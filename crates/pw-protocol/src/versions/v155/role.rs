use crate::octets::OctetsStream;
use pw_core::CharacterSummary;

/// Encodifica RoleInfo na versão 1.5.5 (23 campos completos)
pub fn encode_role_info_155(stream: &mut OctetsStream, c: &CharacterSummary) {
    stream.write_i32(c.id);
    stream.write_u8(c.gender as u8);
    stream.write_u8(c.race as u8);
    stream.write_u8(c.cls as u8);
    stream.write_i32(c.level);
    stream.write_i32(0); // level2
    stream.write_string_utf16le(&c.name);

    let appearance_bytes = if let Some(raw_hex) = c.custom_appearance.get("raw").and_then(|v| v.as_str()) {
        hex::decode(raw_hex).unwrap_or_default()
    } else {
        serde_json::to_vec(&c.custom_appearance).unwrap_or_default()
    };
    stream.write_octets(&appearance_bytes);

    stream.write_compact_uint(c.equipment.len() as u32);
    for item in &c.equipment {
        stream.write_u32(item.item_id);
        stream.write_i32(item.slot as i32);
        stream.write_i32(item.count as i32);
        stream.write_i32(item.max_count as i32);
        stream.write_octets(&[]); // data
        stream.write_i32(0);     // proctype
        stream.write_i32(0);     // expire_date
        stream.write_i32(0);     // guid1
        stream.write_i32(0);     // guid2
        stream.write_i32(0);     // mask
    }

    stream.write_i8(if c.is_deleted { 2 } else { 1 });  // status
    stream.write_i32(0);  // delete_time
    stream.write_i32(0);  // create_time
    stream.write_i32(
        c.last_login_at
            .map(|t| t.timestamp().clamp(0, i32::MAX as i64) as i32)
            .unwrap_or(0),
    );
    stream.write_f32(c.position.x);
    stream.write_f32(c.position.y);
    stream.write_f32(c.position.z);
    stream.write_i32(c.world_id);
    stream.write_octets(&[]); // custom_status
    stream.write_octets(&[]); // charactermode

    // Campos introduzidos no 1.4.8+ / 1.5.5
    stream.write_i32(0);      // referrer_role
    stream.write_i32(0);      // cash_add
    stream.write_octets(&[]); // reincarnation_data
    stream.write_octets(&[]); // realm_data
}
