Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/delivery.ps1" -LibraryOnly
$nativeMergeFunction = ${function:Request-DeliveryAutoMerge}

# No network, credentials, fetched code, or third-party test modules required.
function Invoke-DeliveryApi { throw 'Offline fixtures must never access GitHub.' }
function Request-DeliveryAutoMerge { throw 'Offline fixtures must never mutate GitHub.' }
$script:Assertions = 0
function Assert-Delivery($Actual, $Expected, [string]$Name) {
    if ($Actual -cne $Expected) { throw "$Name`: expected [$Expected], got [$Actual]." }
    $script:Assertions++
}
function New-DeliveryPull {
    return [pscustomobject]@{
        number = 42; state = 'open'; draft = $false; merged = $false
        base = [pscustomobject]@{ ref = 'main' }
        head = [pscustomobject]@{ sha = ('a' * 40); repo = [pscustomobject]@{ full_name = 'fixture/repo' } }
        labels = @([pscustomobject]@{ name = 'delivery:auto-merge' })
        mergeable = $true; mergeable_state = 'clean'; auto_merge = $null
        updated_at = '2026-11-02T10:00:00Z'
    }
}
function New-DeliveryCheck([string]$Name, [string]$Conclusion = 'success', [string]$Sha = ('a' * 40)) {
    return [pscustomobject]@{ name = $Name; head_sha = $Sha; status = 'completed'; conclusion = $Conclusion; app = [pscustomobject]@{ id = 15368 } }
}
$repo = 'fixture/repo'
$now = [datetime]'2026-11-02T11:00:00Z'
$pull = New-DeliveryPull
$checks = @((New-DeliveryCheck 'green-windows'), (New-DeliveryCheck 'green-linux'))
Assert-Delivery (Test-DeliveryScope $pull $repo) $true 'same-repository opted-in scope'
$pull.draft = $true
Assert-Delivery (Test-DeliveryScope $pull $repo) $false 'draft excluded'
$pull = New-DeliveryPull; $pull.head.repo.full_name = 'fork/repo'
Assert-Delivery (Test-DeliveryScope $pull $repo) $false 'fork excluded'
$pull = New-DeliveryPull; $pull.base.ref = 'release'
Assert-Delivery (Test-DeliveryScope $pull $repo) $false 'wrong base excluded'
$pull = New-DeliveryPull; $pull.labels = @()
Assert-Delivery (Test-DeliveryScope $pull $repo) $false 'unlabelled excluded'
$pull = New-DeliveryPull; $pull.head.sha = 'invalid'
Assert-Delivery (Test-DeliveryScope $pull $repo) $false 'invalid head excluded'
$pull = New-DeliveryPull
Assert-Delivery (Get-DeliveryDecision $pull $repo $checks @() $now) 'ready' 'required checks observed'
$pull.mergeable = $false
Assert-Delivery (Get-DeliveryDecision $pull $repo $checks @() $now) 'conflict' 'conflict wins'
$pull = New-DeliveryPull
$failed = @((New-DeliveryCheck 'green-windows' 'failure'), (New-DeliveryCheck 'green-linux'))
Assert-Delivery (Get-DeliveryDecision $pull $repo $failed @() $now) 'checks-failed' 'current required failure'
$stale = $checks + @((New-DeliveryCheck 'green-windows' 'failure' ('b' * 40)))
Assert-Delivery (Get-DeliveryDecision $pull $repo $stale @() $now) 'ready' 'stale head failure ignored'
$wrongApp = New-DeliveryCheck 'green-windows' 'failure'; $wrongApp.app.id = 1
Assert-Delivery (Get-DeliveryDecision $pull $repo ($checks + @($wrongApp)) @() $now) 'ready' 'foreign app cannot spoof required failure'
Assert-Delivery (Get-DeliveryDecision $pull $repo @() @() $now) 'missing-ci' 'stalled missing checks'
$runs = @([pscustomobject]@{ head_sha = $pull.head.sha; status = 'queued'; conclusion = $null })
Assert-Delivery (Get-DeliveryDecision $pull $repo @() $runs $now) 'waiting' 'queued CI waits'
$runs[0].status = 'waiting'
Assert-Delivery (Get-DeliveryDecision $pull $repo @() $runs $now) 'approval-needed' 'approval is actionable'
$oldApproval = [pscustomobject]@{ id = 1; head_sha = $pull.head.sha; status = 'waiting'; conclusion = $null }
$newRun = [pscustomobject]@{ id = 2; head_sha = $pull.head.sha; status = 'queued'; conclusion = $null }
Assert-Delivery (Get-DeliveryDecision $pull $repo @() @($oldApproval, $newRun) $now) 'waiting' 'newer run replaces old approval wait'
$pull.mergeable_state = 'behind'
Assert-Delivery (Get-DeliveryDecision $pull $repo @() @() $now) 'update' 'normal behind update'
$pull.mergeable = $null
Assert-Delivery (Get-DeliveryDecision $pull $repo @() @() $now) 'waiting' 'unknown mergeability waits'
$pull = New-DeliveryPull; $fresh = New-DeliveryPull
Assert-Delivery (Get-DeliveryMutation 'ready' $pull $fresh $repo) 'auto-merge' 'eligible native request'
$fresh.head.sha = ('b' * 40)
Assert-Delivery (Get-DeliveryMutation 'ready' $pull $fresh $repo) 'none' 'changed head blocks merge request'
$fresh = New-DeliveryPull; $fresh.labels = @()
Assert-Delivery (Get-DeliveryMutation 'ready' $pull $fresh $repo) 'none' 'withdrawn label blocks mutation'
$fresh = New-DeliveryPull; $fresh.auto_merge = @{ merge_method = 'squash' }
Assert-Delivery (Get-DeliveryMutation 'ready' $pull $fresh $repo) 'none' 'existing auto-merge is idempotent'
$fresh = New-DeliveryPull; $fresh.mergeable_state = 'behind'
Assert-Delivery (Get-DeliveryMutation 'update' $pull $fresh $repo) 'update' 'matching head update'
$fresh.head.sha = ('b' * 40)
Assert-Delivery (Get-DeliveryMutation 'update' $pull $fresh $repo) 'none' 'changed head blocks update'
$create = Get-DeliveryIssuePlan $null 42 $repo $pull.head.sha 'conflict'
Assert-Delivery $create.action 'create' 'new blocker'
$issue = [pscustomobject]@{ number = 100; state = 'open'; body = $create.body }
Assert-Delivery (Get-DeliveryIssuePlan $issue 42 $repo $pull.head.sha 'conflict').action 'none' 'identical alert deduplicated'
$issue.body = "Operator notes remain.`n" + $issue.body + "`nMore notes."
$clear = Get-DeliveryIssuePlan $issue 42 $repo $pull.head.sha 'waiting'
Assert-Delivery $clear.state 'closed' 'cleared blocker closes while checks may wait'
Assert-Delivery ($clear.body.StartsWith('Operator notes remain.')) $true 'leading notes preserved'
Assert-Delivery ($clear.body.EndsWith('More notes.')) $true 'trailing notes preserved'
$issue.state = 'closed'; $issue.body = $clear.body
Assert-Delivery (Get-DeliveryIssuePlan $issue 42 $repo $pull.head.sha 'conflict').state 'open' 'recurrence reopens same issue'
Assert-Delivery (Get-DeliveryIssuePlan $issue 42 $repo $pull.head.sha 'merged').state_reason 'completed' 'merged blocker completion'
Assert-Delivery (Get-DeliveryIssuePlan $issue 42 $repo $pull.head.sha 'closed').state_reason 'not_planned' 'unmerged closure is not delivery'
Assert-Delivery (Get-DeliveryIssuePlan $issue 42 $repo $pull.head.sha 'out-of-scope').state_reason 'not_planned' 'withdrawn scope retirement'
Assert-Delivery (Get-DeliveryIssuePlan $null 42 $repo $pull.head.sha 'waiting').action 'none' 'pending is not a new blocker'
$pull.state = 'closed'; $pull.merged = $true
Assert-Delivery (Get-DeliveryDecision $pull $repo) 'merged' 'closed merged PR tracked'
$pull.merged = $false
Assert-Delivery (Get-DeliveryDecision $pull $repo) 'closed' 'closed unmerged PR tracked'
$sha = 'a' * 40
Assert-Delivery (Get-DeliveryMainDecision @() @() $false $sha) 'dispatch' 'missing main build dispatch'
Assert-Delivery (Get-DeliveryMainDecision @() @() $true $sha) 'dispatch-recorded' 'durable marker prevents duplicate dispatch'
$mainRun = [pscustomobject]@{ id = 10; head_sha = $sha; head_branch = 'main'; status = 'in_progress'; conclusion = $null }
Assert-Delivery (Get-DeliveryMainDecision @($mainRun) @() $false $sha) 'building' 'existing main run deduplicates dispatch'
$mainRun.status = 'completed'; $mainRun.conclusion = 'failure'
Assert-Delivery (Get-DeliveryMainDecision @($mainRun) @() $false $sha) 'build-failed' 'failed main is not test ready'
$mainRun.conclusion = 'success'
Assert-Delivery (Get-DeliveryMainDecision @($mainRun) @() $false $sha) 'artifact-unavailable' 'success without artifact is explicit'
$artifact = [pscustomobject]@{ name = "devpulselabs-windows-$sha"; expired = $false }
Assert-Delivery (Get-DeliveryMainDecision @($mainRun) @($artifact) $false $sha) 'artifact-ready' 'matching available executable'
$artifact.expired = $true
Assert-Delivery (Get-DeliveryMainDecision @($mainRun) @($artifact) $false $sha) 'artifact-unavailable' 'expired executable is unavailable'
$mainRun.head_branch = 'feature'
Assert-Delivery (Get-DeliveryMainDecision @($mainRun) @() $false $sha) 'dispatch' 'PR run cannot stand in for main artifact run'
# Exercise orchestration against in-memory API responses, including dispatch
# idempotence. The fake rejects every endpoint outside this fixture.
$script:Dispatches = 0
$script:DispatchMarker = $false
$script:FixtureMainSha = $sha
$script:FixtureCalls = New-Object 'System.Collections.Generic.List[string]'
function Invoke-DeliveryApi([string]$Method, [string]$Path, $Body = $null) {
    $script:FixtureCalls.Add("$Method $Path")
    if ($Method -eq 'GET' -and $Path -eq 'repos/fixture/repo/git/ref/heads/main') {
        return [pscustomobject]@{ object = [pscustomobject]@{ sha = $script:FixtureMainSha } }
    }
    if ($Method -eq 'GET' -and $Path.StartsWith('repos/fixture/repo/pulls?')) {
        return [pscustomobject]@{ number = 42; merge_commit_sha = $script:FixtureMainSha; merged_at = '2026-11-02T10:00:00Z'; head = [pscustomobject]@{ repo = [pscustomobject]@{ full_name = 'fixture/repo' } }; labels = @([pscustomobject]@{ name = 'delivery:auto-merge' }) }
    }
    if ($Method -eq 'GET' -and $Path.StartsWith('repos/fixture/repo/actions/workflows/green.yml/runs?')) {
        return [pscustomobject]@{ workflow_runs = @() }
    }
    if ($Method -eq 'GET' -and $Path.StartsWith('repos/fixture/repo/issues/42/comments?')) {
        if ($script:DispatchMarker) { return [pscustomobject]@{ body = "<!-- devpulse-delivery-main-dispatch:v1:$script:FixtureMainSha -->" } }
        return @()
    }
    if ($Method -eq 'POST' -and $Path -eq 'repos/fixture/repo/issues/42/comments') {
        $script:DispatchMarker = $true
        return [pscustomobject]@{ id = 7 }
    }
    if ($Method -eq 'POST' -and $Path -eq 'repos/fixture/repo/actions/workflows/green.yml/dispatches') {
        Assert-Delivery $Body.ref 'main' 'orchestrated dispatch targets trusted main only'
        Assert-Delivery $script:DispatchMarker $true 'intent is persisted before dispatch'
        $script:Dispatches++
        return $null
    }
    throw "Unexpected offline API call: $Method $Path"
}
$summary = New-Object 'System.Collections.Generic.List[string]'
Invoke-DeliveryMain $repo $summary
Invoke-DeliveryMain $repo $summary
Assert-Delivery $script:Dispatches 1 'repeated reconciliation dispatches exactly once'
Assert-Delivery ($summary[1] -match 'dispatch-recorded') $true 'repeat reports intent without pretending success'

function gh {
    $script:MergeArguments = @($args)
    $global:LASTEXITCODE = 0
}
& $nativeMergeFunction $repo 42 $sha
Assert-Delivery ($script:MergeArguments -join ' ') "pr merge 42 --repo fixture/repo --auto --squash --match-head-commit $sha" 'native merge uses exact-head guard and no bypass'
Write-Output "Delivery offline fixtures passed: $script:Assertions assertions."
