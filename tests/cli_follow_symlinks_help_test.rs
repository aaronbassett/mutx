use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_follow_symlinks_help_text_does_not_mention_housekeep() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    let output = cmd.arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the --follow-symlinks section (may span multiple lines)
    let help_text: String = stdout
        .lines()
        .skip_while(|line| !line.contains("--follow-symlinks"))
        .take_while(|line| {
            line.contains("--follow-symlinks")
                || (!line.trim().starts_with("--") && !line.trim().is_empty())
        })
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        !help_text.is_empty(),
        "Should find --follow-symlinks in help output"
    );

    // The help text should NOT mention "housekeep operations"
    assert!(
        !help_text.contains("housekeep operations") && !help_text.contains("housekeep"),
        "Help text for --follow-symlinks should not mention 'housekeep', but found: {}",
        help_text
    );

    // Should mention output files
    assert!(
        help_text.contains("output") || help_text.contains("file"),
        "Help text for --follow-symlinks should mention output or files, but found: {}",
        help_text
    );
}

#[test]
fn test_follow_symlinks_help_text_is_accurate() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Follow symbolic links for output files",
        ));
}
