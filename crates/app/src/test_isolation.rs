//! Test-only process isolation for the process-wide library authority.

pub(super) fn chrome_retry_in_child() -> bool {
    if std::env::var_os("DPL_CHROME_RETRY_TEST_CHILD").is_some() {
        return false;
    }
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "tests::chrome_choices_retry_a_failed_save_when_storage_returns",
        ])
        .env("DPL_CHROME_RETRY_TEST_CHILD", "1");
    let output = dpl_proc::bounded_output(
        dpl_proc::quiet(&mut command),
        std::time::Duration::from_secs(15),
        16 * 1024,
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
