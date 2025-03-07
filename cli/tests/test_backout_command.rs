// Copyright 2024 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;

use crate::common::CommandOutput;
use crate::common::TestEnvironment;

fn create_commit(
    test_env: &TestEnvironment,
    repo_path: &Path,
    name: &str,
    parents: &[&str],
    files: &[(&str, &str)],
) {
    let parents = match parents {
        [] => &["root()"],
        parents => parents,
    };
    test_env
        .run_jj_with(|cmd| {
            cmd.current_dir(repo_path)
                .args(["new", "-m", name])
                .args(parents)
        })
        .success();
    for (name, contents) in files {
        std::fs::write(repo_path.join(name), contents).unwrap();
    }
    test_env
        .run_jj_in(repo_path, ["bookmark", "create", "-r@", name])
        .success();
}

#[test]
fn test_backout() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    create_commit(&test_env, &repo_path, "a", &[], &[("a", "a\n")]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s"]);
    insta::assert_snapshot!(stdout, @r"
    A a
    [EOF]
    ");

    // Backout the commit
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["backout", "-r", "@"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r#"
    ○  6d845ed9fb6a Back out "a"
    │
    │  This backs out commit 2443ea76b0b1c531326908326aab7020abab8e6c.
    @  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    "#);
    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s", "-r", "@+"]);
    insta::assert_snapshot!(stdout, @r"
    D a
    [EOF]
    ");

    // Backout the new backed-out commit
    test_env.jj_cmd_ok(&repo_path, &["edit", "@+"]);
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["backout", "-r", "@"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r#"
    ○  79555ea9040b Back out "Back out "a""
    │
    │  This backs out commit 6d845ed9fb6a3d367e2d7068ef0256b1a10705a9.
    @  6d845ed9fb6a Back out "a"
    │
    │  This backs out commit 2443ea76b0b1c531326908326aab7020abab8e6c.
    ○  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    "#);
    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s", "-r", "@+"]);
    insta::assert_snapshot!(stdout, @r"
    A a
    [EOF]
    ");
}

#[test]
fn test_backout_multiple() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    create_commit(&test_env, &repo_path, "a", &[], &[("a", "a\n")]);
    create_commit(&test_env, &repo_path, "b", &["a"], &[("a", "a\nb\n")]);
    create_commit(
        &test_env,
        &repo_path,
        "c",
        &["b"],
        &[("a", "a\nb\n"), ("b", "b\n")],
    );
    create_commit(&test_env, &repo_path, "d", &["c"], &[]);
    create_commit(&test_env, &repo_path, "e", &["d"], &[("a", "a\nb\nc\n")]);

    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  208f8612074a e
    ○  ceeec03be46b d
    ○  413337bbd11f c
    ○  46cc97af6802 b
    ○  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    ");

    // Backout multiple commits
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["backout", "-r", "b", "-r", "c", "-r", "e"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r#"
    ○  6504c4ded177 Back out "b"
    │
    │  This backs out commit 46cc97af6802301d8db381386e8485ff3ff24ae6.
    ○  d31d42e0267f Back out "c"
    │
    │  This backs out commit 413337bbd11f7a6636c010d9e196acf801d8df2f.
    ○  8ff3fbc2ccb0 Back out "e"
    │
    │  This backs out commit 208f8612074af4c219d06568a8e1f04f2e80dc25.
    @  208f8612074a e
    ○  ceeec03be46b d
    ○  413337bbd11f c
    ○  46cc97af6802 b
    ○  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    "#);
    // View the output of each backed out commit
    let stdout = test_env.jj_cmd_success(&repo_path, &["show", "@+"]);
    insta::assert_snapshot!(stdout, @r#"
    Commit ID: 8ff3fbc2ccb0d66985f558c461d1643cebb4c7d6
    Change ID: wqnwkozpkustnxypnnntnykwrqrkrpvv
    Author   : Test User <test.user@example.com> (2001-02-03 08:05:19)
    Committer: Test User <test.user@example.com> (2001-02-03 08:05:19)

        Back out "e"

        This backs out commit 208f8612074af4c219d06568a8e1f04f2e80dc25.

    Modified regular file a:
       1    1: a
       2    2: b
       3     : c
    [EOF]
    "#);
    let stdout = test_env.jj_cmd_success(&repo_path, &["show", "@++"]);
    insta::assert_snapshot!(stdout, @r#"
    Commit ID: d31d42e0267f6524d445348b1dd00926c62a6b57
    Change ID: mouksmquosnpvwqrpsvvxtxpywpnxlss
    Author   : Test User <test.user@example.com> (2001-02-03 08:05:19)
    Committer: Test User <test.user@example.com> (2001-02-03 08:05:19)

        Back out "c"

        This backs out commit 413337bbd11f7a6636c010d9e196acf801d8df2f.

    Removed regular file b:
       1     : b
    [EOF]
    "#);
    let stdout = test_env.jj_cmd_success(&repo_path, &["show", "@+++"]);
    insta::assert_snapshot!(stdout, @r#"
    Commit ID: 6504c4ded177fba2334f76683d1aa643700d5073
    Change ID: tqvpomtpwrqsylrpsxknultrymmqxmxv
    Author   : Test User <test.user@example.com> (2001-02-03 08:05:19)
    Committer: Test User <test.user@example.com> (2001-02-03 08:05:19)

        Back out "b"

        This backs out commit 46cc97af6802301d8db381386e8485ff3ff24ae6.

    Modified regular file a:
       1    1: a
       2     : b
    [EOF]
    "#);
}

#[test]
fn test_backout_description_template() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    test_env.add_config(
        r#"
        [templates]
        backout_description = '''
        separate(" ",
          "Revert commit",
          commit_id.short(),
          '"' ++ description.first_line() ++ '"',
        )
        '''
        "#,
    );
    let repo_path = test_env.env_root().join("repo");
    create_commit(&test_env, &repo_path, "a", &[], &[("a", "a\n")]);

    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "-s"]);
    insta::assert_snapshot!(stdout, @r"
    A a
    [EOF]
    ");

    // Verify that message of backed out commit follows the template
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["backout", "-r", "a"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r#"
    ○  1db880a5204e Revert commit 2443ea76b0b1 "a"
    @  2443ea76b0b1 a
    ◆  000000000000
    [EOF]
    "#);
}

#[must_use]
fn get_log_output(test_env: &TestEnvironment, cwd: &Path) -> CommandOutput {
    let template = r#"commit_id.short() ++ " " ++ description"#;
    test_env.run_jj_in(cwd, ["log", "-T", template])
}
