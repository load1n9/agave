pub use lite_json::json_parser::parse_json;
pub use lite_json::JsonValue;

pub fn parse_json_file(path: &str) -> JsonValue {
    let data = super::fs::read_to_string(path).expect("Could not read file!");
    parse_json(data.as_str()).expect("Could not parse JSON!")
}
