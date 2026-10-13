use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn persistent_requests_match_one_shot_and_exit_on_eof() {
    let binary = env!("CARGO_BIN_EXE_orient");
    let mut server = Command::new(binary)
        .args(["--rpc", "--serve"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = server.stdin.take().unwrap();
    let mut output = BufReader::new(server.stdout.take().unwrap());
    for (id, operation, payload) in [
        (json!(17), "lch", json!({"value":"aabbcc"})),
        (json!("two"), "clean", json!({"value":"bad"})),
        (
            json!(19),
            "render_values",
            json!({"template":"λ\n{{color}}", "values":{"color":"abcdef"}}),
        ),
        (
            json!(20),
            "render_values",
            json!({"template":"x".repeat(200_000), "values":{}}),
        ),
    ] {
        let request = json!({"id":id,"operation":operation,"payload":payload});
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let mut reply: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["id"], id);
        reply.as_object_mut().unwrap().remove("id");
        let mut once = Command::new(binary)
            .arg("--rpc")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        write!(once.stdin.take().unwrap(), "{request}").unwrap();
        let result = once.wait_with_output().unwrap();
        assert_eq!(
            reply,
            serde_json::from_slice::<Value>(&result.stdout).unwrap()
        );
    }
    writeln!(input, "not-json").unwrap();
    input.flush().unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert!(reply["id"].is_null());
    assert!(reply["error"].is_object());
    writeln!(
        input,
        "{}",
        json!({"id":21,"operation":"info","payload":{}})
    )
    .unwrap();
    input.flush().unwrap();
    line.clear();
    output.read_line(&mut line).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&line).unwrap()["id"], 21);
    drop(input);
    assert!(server.wait().unwrap().success());
    line.clear();
    assert_eq!(output.read_line(&mut line).unwrap(), 0);
}
