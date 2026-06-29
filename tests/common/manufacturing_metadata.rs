use serde_yaml_ng::Value;

pub fn remove_board_manufacturing(project_yaml: &mut Value) {
    let board = project_yaml["board"].as_mapping_mut().unwrap();
    board.remove(Value::String("manufacturing".to_string()));
}

pub fn remove_layout_stackup(project_yaml: &mut Value) {
    let board = project_yaml["board"].as_mapping_mut().unwrap();
    let layout = board
        .get_mut(Value::String("layout".to_string()))
        .unwrap()
        .as_mapping_mut()
        .unwrap();
    layout.remove(Value::String("stackup".to_string()));
}

pub fn assert_runnable<'a>(suggestions: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], true, "{id}");
    assert!(suggestion.get("required_inputs").is_none(), "{id}");
    suggestion
}
