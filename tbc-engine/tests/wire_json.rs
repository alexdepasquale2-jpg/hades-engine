use serde::Deserialize;
use tbc_engine::wire_json::compat;

#[derive(Deserialize)]
struct MoveBody {
    #[serde(with = "compat")]
    fwau: u128,
    dx: f32,
    dy: f32,
}

#[test]
fn deser_string_fwau() {
    let body = r#"{"fwau":"12345","dx":1.0,"dy":0.0}"#;
    let m: MoveBody = serde_json::from_str(body).expect("parse");
    assert_eq!(m.fwau, 12345);
}

#[test]
fn deser_number_fwau() {
    let body = r#"{"fwau":12345,"dx":1.0,"dy":0.0}"#;
    let m: MoveBody = serde_json::from_str(body).expect("parse");
    assert_eq!(m.fwau, 12345);
}
