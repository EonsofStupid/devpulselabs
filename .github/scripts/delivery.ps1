[CmdletBinding()]
param([string]$Repository = $env:GITHUB_REPOSITORY, [switch]$LibraryOnly)

# This file is trusted-main CI infrastructure, never a product runtime service.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-DeliveryValue($Object, [string]$Name, $Default = $null) {
    if ($null -eq $Object) { return $Default }
    if ($Object -is [System.Collections.IDictionary]) {
        if ($Object.Contains($Name)) { return $Object[$Name] }
    } elseif ($null -ne $Object.PSObject.Properties[$Name]) { return $Object.$Name }
    return $Default
}

function Invoke-DeliveryApi([string]$Method, [string]$Path, $Body = $null) {
    $arguments = @('api', '--method', $Method, '-H', 'Accept: application/vnd.github+json', $Path)
    $temporaryPath = $null
    try {
        if ($null -ne $Body) {
            $temporaryPath = [IO.Path]::GetTempFileName()
            [IO.File]::WriteAllText($temporaryPath, ($Body | ConvertTo-Json -Depth 20 -Compress), (New-Object Text.UTF8Encoding($false)))
            $arguments += @('--input', $temporaryPath)
        }
        $output = & gh @arguments 2>&1
        if ($LASTEXITCODE -ne 0) { throw "GitHub API $Method failed for $Path (exit $LASTEXITCODE)." }
        if ($output) { return (($output -join "`n") | ConvertFrom-Json) }
    } finally {
        if ($temporaryPath) { Remove-Item -LiteralPath $temporaryPath -Force }
    }
}

function Get-DeliveryPages([string]$Path, [string]$Property = '') {
    $separator = '?'
    if ($Path.Contains('?')) { $separator = '&' }
    for ($page = 1; $page -le 100; $page++) {
        $response = Invoke-DeliveryApi GET ("{0}{1}per_page=100&page={2}" -f $Path, $separator, $page)
        $items = @($response)
        if ($Property) { $items = @(Get-DeliveryValue $response $Property @()) }
        foreach ($item in $items) { if ($null -ne $item) { $item } }
        if ($items.Count -lt 100) { return }
    }
    throw 'Delivery pagination exceeded its safety bound; no partial result accepted.'
}

function Test-DeliveryScope($Pull, [string]$Repository) {
    return ((Get-DeliveryValue $Pull state) -eq 'open' -and
        -not (Get-DeliveryValue $Pull draft $true) -and
        (Get-DeliveryValue (Get-DeliveryValue $Pull base) ref) -eq 'main' -and
        (Get-DeliveryValue (Get-DeliveryValue (Get-DeliveryValue $Pull head) repo) full_name) -eq $Repository -and
        @((Get-DeliveryValue $Pull labels @()) | ForEach-Object { Get-DeliveryValue $_ name }) -contains 'delivery:auto-merge' -and
        (Get-DeliveryValue (Get-DeliveryValue $Pull head) sha '') -match '^[a-fA-F0-9]{40}$')
}

function Get-DeliveryDecision($Pull, [string]$Repository, [array]$Checks = @(), [array]$Runs = @(), [datetime]$Now = [datetime]::UtcNow) {
    if ((Get-DeliveryValue $Pull state) -eq 'closed') {
        if (Get-DeliveryValue $Pull merged $false) { return 'merged' }
        return 'closed'
    }
    if (-not (Test-DeliveryScope $Pull $Repository)) { return 'out-of-scope' }
    $mergeable = Get-DeliveryValue $Pull mergeable
    $mergeState = Get-DeliveryValue $Pull mergeable_state
    if ($mergeable -eq $false -or $mergeState -eq 'dirty') { return 'conflict' }
    $sha = $Pull.head.sha
    $current = @($Checks | Where-Object { (Get-DeliveryValue $_ head_sha) -eq $sha -and
        (Get-DeliveryValue (Get-DeliveryValue $_ app) id) -eq 15368 -and
        (Get-DeliveryValue $_ name) -in @('green-windows', 'green-linux') })
    if (@($current | Where-Object { (Get-DeliveryValue $_ conclusion) -in @('failure', 'timed_out', 'cancelled', 'startup_failure', 'action_required', 'stale') }).Count) { return 'checks-failed' }
    # A later workflow replaces an older approval/action-required observation.
    $currentRuns = @($Runs | Where-Object { (Get-DeliveryValue $_ head_sha) -eq $sha } |
        Sort-Object { Get-DeliveryValue $_ id 0 } -Descending | Select-Object -First 1)
    if (@($currentRuns | Where-Object { (Get-DeliveryValue $_ status) -eq 'waiting' -or (Get-DeliveryValue $_ conclusion) -eq 'action_required' }).Count) { return 'approval-needed' }
    if ($mergeState -eq 'behind' -and $mergeable -eq $true) { return 'update' }
    if ($null -eq $mergeable -or $mergeState -eq 'unknown') { return 'waiting' }
    $done = @($current | Where-Object { (Get-DeliveryValue $_ status) -eq 'completed' -and (Get-DeliveryValue $_ conclusion) -in @('success', 'neutral', 'skipped') } | Select-Object -ExpandProperty name -Unique)
    if ($done.Count -eq 2) { return 'ready' }
    $active = @($currentRuns | Where-Object { (Get-DeliveryValue $_ status) -in @('queued', 'requested', 'in_progress', 'pending') })
    $updated = [datetime](Get-DeliveryValue $Pull updated_at $Now)
    if ($current.Count -lt 2 -and $active.Count -eq 0 -and ($Now.ToUniversalTime() - $updated.ToUniversalTime()).TotalMinutes -ge 30) { return 'missing-ci' }
    return 'waiting'
}

function Get-DeliveryBlockerText([string]$Reason) {
    switch ($Reason) {
        'conflict' { return 'Merge conflict: resolve the branch normally, push, and let both required green checks run.' }
        'checks-failed' { return 'A required green check on the current head failed or needs attention. Fix the failure or explicitly retry it.' }
        'approval-needed' { return 'The current green workflow is waiting for approval or action. A writer must inspect and approve eligible trusted CI in GitHub.' }
        'missing-ci' { return 'Required current-head green checks are missing after 30 minutes with no active run. Check the Actions approval banner and run eligibility; do not bypass protection.' }
        'merged' { return 'The pull request merged. This blocker is complete; executable availability is reported separately by the delivery workflow.' }
        'closed' { return 'The pull request closed without merging. This blocker is retired, not a completed delivery.' }
        'out-of-scope' { return 'This pull request is no longer opted into eligible delivery. This blocker is retired; pending native auto-merge is managed separately in GitHub.' }
        default { return 'The previous blocker cleared. Delivery may still await checks, review, or other GitHub protection requirements.' }
    }
}

function Get-DeliveryIssuePlan($Issue, [int]$Number, [string]$Repository, [string]$Sha, [string]$Reason) {
    $start = "<!-- devpulse-delivery-blocker:v1:$Number`:start -->"
    $end = "<!-- devpulse-delivery-blocker:v1:$Number`:end -->"
    $block = "$start`n$(Get-DeliveryBlockerText $Reason)`n`nPull request: https://github.com/$Repository/pull/$Number`nObserved head: $Sha`n$end"
    $blocked = $Reason -in @('conflict', 'checks-failed', 'approval-needed', 'missing-ci')
    if ($null -eq $Issue) {
        if ($blocked) { return @{ action = 'create'; body = $block; state = 'open'; title = "Delivery blocker for PR #$Number" } }
        return @{ action = 'none' }
    }
    $body = [string](Get-DeliveryValue $Issue body '')
    $pattern = '(?s)' + [regex]::Escape($start) + '.*?' + [regex]::Escape($end)
    if (-not [regex]::IsMatch($body, $pattern)) { throw 'Refusing to edit an issue without its complete managed marker.' }
    $replacement = [regex]::Replace($body, $pattern, [System.Text.RegularExpressions.MatchEvaluator]{ param($match) $block })
    $state = 'closed'
    $stateReason = 'completed'
    if ($blocked) { $state = 'open'; $stateReason = 'reopened' }
    if ($Reason -in @('closed', 'out-of-scope')) { $stateReason = 'not_planned' }
    if ((Get-DeliveryValue $Issue state) -eq $state -and $replacement -eq $body) { return @{ action = 'none' } }
    return @{ action = 'update'; body = $replacement; state = $state; state_reason = $stateReason }
}

function Sync-DeliveryIssue($Issue, [int]$Number, [string]$Repository, [string]$Sha, [string]$Reason) {
    $plan = Get-DeliveryIssuePlan $Issue $Number $Repository $Sha $Reason
    if ($plan.action -eq 'create') {
        $null = Invoke-DeliveryApi POST "repos/$Repository/issues" @{ title = $plan.title; body = $plan.body; labels = @('delivery:blocker') }
    } elseif ($plan.action -eq 'update') {
        $null = Invoke-DeliveryApi PATCH "repos/$Repository/issues/$($Issue.number)" @{ body = $plan.body; state = $plan.state; state_reason = $plan.state_reason }
    }
}

function Get-DeliveryMutation([string]$Decision, $Observed, $Fresh, [string]$Repository) {
    if (-not (Test-DeliveryScope $Fresh $Repository) -or $Observed.head.sha -ne $Fresh.head.sha) { return 'none' }
    if ($Decision -eq 'update' -and $Fresh.mergeable -eq $true -and $Fresh.mergeable_state -eq 'behind') { return 'update' }
    if ($Decision -in @('ready', 'waiting') -and $Fresh.mergeable -ne $false -and -not (Get-DeliveryValue $Fresh auto_merge)) { return 'auto-merge' }
    return 'none'
}

function Request-DeliveryAutoMerge([string]$Repository, [int]$Number, [string]$Sha) {
    # --match-head-commit closes the read/mutation race. No admin or bypass option.
    $output = & gh pr merge "$Number" --repo $Repository --auto --squash --match-head-commit $Sha 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Native protected auto-merge request failed for PR #$Number (exit $LASTEXITCODE)." }
}

function Get-DeliveryMainDecision([array]$Runs, [array]$Artifacts, [bool]$HasDispatchMarker, [string]$Sha) {
    $current = @($Runs | Where-Object { $_.head_sha -eq $Sha -and $_.head_branch -eq 'main' } | Sort-Object id -Descending)
    if ($current.Count -eq 0) {
        if ($HasDispatchMarker) { return 'dispatch-recorded' }
        return 'dispatch'
    }
    if ($current[0].status -ne 'completed') { return 'building' }
    if ($current[0].conclusion -ne 'success') { return 'build-failed' }
    if (@($Artifacts | Where-Object { $_.name -eq "devpulselabs-windows-$Sha" -and -not $_.expired }).Count) { return 'artifact-ready' }
    return 'artifact-unavailable'
}

function Invoke-DeliveryMain([string]$Repository, [System.Collections.Generic.List[string]]$Summary) {
    $main = Invoke-DeliveryApi GET "repos/$Repository/git/ref/heads/main"
    $sha = $main.object.sha
    # Only dispatch for a tracked merge that is still the current main head.
    $closed = @(Get-DeliveryPages "repos/$Repository/pulls?state=closed&base=main&sort=updated&direction=desc")
    $tracked = @($closed | Where-Object { $_.merge_commit_sha -eq $sha -and $null -ne $_.merged_at -and
        (Get-DeliveryValue (Get-DeliveryValue $_.head repo) full_name) -eq $Repository -and
        @($_.labels | ForEach-Object { $_.name }) -contains 'delivery:auto-merge' })
    if ($tracked.Count -eq 0) { $Summary.Add('Main: no opted-in merge at the current head; no build dispatched.'); return }
    $pull = $tracked[0]
    $runs = @(Get-DeliveryPages "repos/$Repository/actions/workflows/green.yml/runs?head_sha=$sha&branch=main" workflow_runs)
    $marker = "<!-- devpulse-delivery-main-dispatch:v1:$sha -->"
    $comments = @(Get-DeliveryPages "repos/$Repository/issues/$($pull.number)/comments")
    $hasMarker = @($comments | Where-Object { ([string]$_.body).Contains($marker) }).Count -gt 0
    $artifacts = @()
    $current = @($runs | Where-Object { $_.head_sha -eq $sha -and $_.head_branch -eq 'main' } | Sort-Object id -Descending)
    if ($current.Count -gt 0 -and $current[0].status -eq 'completed' -and $current[0].conclusion -eq 'success') {
        $artifacts = @(Get-DeliveryPages "repos/$Repository/actions/runs/$($current[0].id)/artifacts" artifacts)
    }
    $decision = Get-DeliveryMainDecision $runs $artifacts $hasMarker $sha
    if ($decision -eq 'dispatch') {
        # Persist intent before dispatch, so a delayed run listing cannot cause duplicates.
        # A crash between these calls requires an explicit operator retry, not a loop.
        $null = Invoke-DeliveryApi POST "repos/$Repository/issues/$($pull.number)/comments" @{ body = "$marker`nMain green build dispatch requested for $sha. If no run appears, inspect delivery Actions and manually dispatch green on main; this marker prevents duplicate requests." }
        $freshMain = Invoke-DeliveryApi GET "repos/$Repository/git/ref/heads/main"
        if ($freshMain.object.sha -ne $sha) { $Summary.Add('Main advanced before dispatch; deferred until the next reconciliation.'); return }
        $null = Invoke-DeliveryApi POST "repos/$Repository/actions/workflows/green.yml/dispatches" @{ ref = 'main' }
    }
    $link = "https://github.com/$Repository/actions/workflows/green.yml"
    if ($current.Count) { $link = "https://github.com/$Repository/actions/runs/$($current[0].id)" }
    $Summary.Add("Main $sha`: $decision. $link")
    if ($decision -eq 'build-failed') { throw 'Main green failed; inspect its linked run before testing or retrying.' }
}

function Invoke-DeliveryMonitor([string]$Repository) {
    if ($Repository -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') { throw 'A valid owner/repository is required.' }
    if (-not $env:GH_TOKEN) { throw 'The ephemeral workflow GH_TOKEN is required.' }
    $summary = New-Object 'System.Collections.Generic.List[string]'
    $errors = New-Object 'System.Collections.Generic.List[string]'
    try {
        $labels = @(Get-DeliveryPages "repos/$Repository/labels")
        foreach ($name in @('delivery:auto-merge', 'delivery:blocker')) {
            if (@($labels | Where-Object { $_.name -eq $name }).Count -eq 0) {
                $null = Invoke-DeliveryApi POST "repos/$Repository/labels" @{ name = $name; color = '8250df'; description = 'Protected delivery automation (explicit opt-in or managed blocker)' }
            }
        }
        $issues = @(Get-DeliveryPages "repos/$Repository/issues?state=all&labels=delivery%3Ablocker&sort=created&direction=desc")
        $byPull = @{}
        foreach ($issue in $issues) {
            if (Get-DeliveryValue $issue pull_request) { continue }
            if ([string]$issue.body -match '<!-- devpulse-delivery-blocker:v1:(\d+):start -->') {
                $number = [int]$Matches[1]
                if (-not $byPull.ContainsKey($number)) { $byPull[$number] = $issue }
            }
        }
        $numbers = @{}
        foreach ($pull in @(Get-DeliveryPages "repos/$Repository/pulls?state=open&base=main")) {
            if (Test-DeliveryScope $pull $Repository) { $numbers[[int]$pull.number] = $true }
        }
        foreach ($number in $byPull.Keys) { if ($byPull[$number].state -eq 'open') { $numbers[$number] = $true } }
        foreach ($number in @($numbers.Keys | Sort-Object)) {
            try {
                $pull = Invoke-DeliveryApi GET "repos/$Repository/pulls/$number"
                $checks = @(); $runs = @()
                $sha = [string](Get-DeliveryValue (Get-DeliveryValue $pull head) sha '')
                if (Test-DeliveryScope $pull $Repository) {
                    $checks = @(Get-DeliveryPages "repos/$Repository/commits/$sha/check-runs?filter=latest" check_runs)
                    $runs = @(Get-DeliveryPages "repos/$Repository/actions/workflows/green.yml/runs?head_sha=$sha" workflow_runs)
                }
                $decision = Get-DeliveryDecision $pull $Repository $checks $runs
                Sync-DeliveryIssue $byPull[$number] $number $Repository $sha $decision
                $summary.Add("PR #$number ($sha): $decision. https://github.com/$Repository/pull/$number")
                if ($decision -in @('update', 'ready', 'waiting')) {
                    $fresh = Invoke-DeliveryApi GET "repos/$Repository/pulls/$number"
                    $mutation = Get-DeliveryMutation $decision $pull $fresh $Repository
                    if ($mutation -eq 'update') {
                        $null = Invoke-DeliveryApi PUT "repos/$Repository/pulls/$number/update-branch" @{ expected_head_sha = $sha }
                        $summary.Add("PR #$number`: ordinary branch update requested; its new checks may require writer approval. No approval or bypass was performed.")
                    } elseif ($mutation -eq 'auto-merge') { Request-DeliveryAutoMerge $Repository $number $sha }
                }
            } catch { $errors.Add("PR #$number`: $($_.Exception.Message)"); $summary.Add($errors[$errors.Count - 1]) }
        }
        try { Invoke-DeliveryMain $Repository $summary } catch { $errors.Add($_.Exception.Message); $summary.Add($_.Exception.Message) }
    } finally {
        $report = "Protected delivery reconciliation`n`n" + (($summary | ForEach-Object { "- $_" }) -join "`n")
        Write-Output $report
        if ($env:GITHUB_STEP_SUMMARY) { [IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, $report + "`n") }
    }
    if ($errors.Count) { throw "Delivery reconciliation had $($errors.Count) operation failure(s); see the summary." }
}

if (-not $LibraryOnly) { Invoke-DeliveryMonitor $Repository }
