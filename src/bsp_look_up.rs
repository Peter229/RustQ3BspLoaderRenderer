pub fn look_up_table(val: &str) -> String {
    match val {
        "textures/skies/pj_dm9sky" => "textures/skies/bluedimclouds".to_string(),
        "textures/base_trim/border12b_pj" => "textures/base_trim/border12b".to_string(), //border12bfx
        "textures/base_wall/glass01" => "textures/effects/tinfx".to_string(),
        "textures/base_button/shootme2" => "textures/base_support/metal3_3".to_string(),
        "textures/base_support/support2rust" => "textures/base_support/support1rust".to_string(),
        "textures/gothic_light/gothic_light3_2K" => "textures/gothic_light/gothic_light3".to_string(), //gothic_light2_blend
        "textures/gothic_light/gothic_light2_2K" => "textures/gothic_light/gothic_light2".to_string(),
        "textures/gothic_light/gothic_light3_3k" => "textures/gothic_light/gothic_light3".to_string(),
        "textures/gothic_light/gothic_light2_lrg_2k" => "textures/gothic_light/gothic_light2_lrg".to_string(),
        "textures/gothic_light/goth_lt2_lrg2k" => "textures/gothic_light/gothic_light2_lrg".to_string(),
        "textures/base_light/proto_light_2k" => "textures/base_light/proto_light".to_string(),
        "textures/base_light/baslt4_1_2k" => "textures/base_light/baslt4_1".to_string(),
        "textures/base_light/patch10_pj_lite2_1000" => "textures/base_light/patch10_pj_lite2".to_string(),
        "textures/sfx/flameanim_green_pj" => "textures/sfx/g_flame1".to_string(),
        "textures/sfx/q3dm9fog" => "textures/liquids/kc_fogcloud3".to_string(),
        "textures/sfx/diamond2cjumppad" => "textures/sfx/bouncepad01b_layer1".to_string(),
        "textures/sfx/teslacoil3" => "textures/sfx/cabletest2".to_string(),
        "textures/liquids/slime1" => "textures/liquids/slime7".to_string(),
        _ => val.to_string(),
    }
}