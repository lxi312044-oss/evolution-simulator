use sim_core::config::SimulationConfig;
#[test]
fn phase32_config_modes_deserialize_and_validate() {
    for mode in [
        "control",
        "ambush",
        "roaming",
        "hybrid",
        "hybrid-evasion",
        "combined80",
        "combined80-evasion",
    ] {
        let path = format!("../../config/phase06232-{mode}.json");
        let data = std::fs::read_to_string(path).unwrap();
        let config: SimulationConfig = serde_json::from_str(&data).unwrap();
        config.validate().unwrap();
    }
}
