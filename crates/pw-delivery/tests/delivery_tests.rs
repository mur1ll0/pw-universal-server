use pw_delivery::chat::ChatChannel;
use pw_delivery::party::{LootRule, PartyManager, PartyMember};

#[tokio::test]
async fn test_party_manager_lifecycle() {
    let party_mgr = PartyManager::new();

    let leader = PartyMember {
        role_id: 1001,
        name: "LiderGuerreiro".to_string(),
        level: 80,
        cls: 0,
        hp: 5000,
        max_hp: 5000,
        mp: 1000,
        max_mp: 1000,
        world_id: 1,
    };

    let party_id = party_mgr.create_party(leader).await;
    assert!(party_id > 0);

    let member2 = PartyMember {
        role_id: 1002,
        name: "MagoAmigo".to_string(),
        level: 75,
        cls: 1,
        hp: 2500,
        max_hp: 2500,
        mp: 4000,
        max_mp: 4000,
        world_id: 1,
    };

    let join_res = party_mgr.add_member(party_id, member2).await;
    assert!(join_res.is_ok());

    let party_opt = party_mgr.get_party_by_player(1002).await;
    assert!(party_opt.is_some());
    let party = party_opt.unwrap();
    assert_eq!(party.members.len(), 2);
    assert_eq!(party.leader_id, 1001);
    assert_eq!(party.loot_rule, LootRule::FreeForAll);

    // Remove membro
    let leave_res = party_mgr.remove_member(1002).await;
    assert!(leave_res.is_some());

    let party_after = party_mgr.get_party_by_player(1002).await;
    assert!(party_after.is_none());
}

#[test]
fn test_chat_channel_parsing() {
    assert_eq!(ChatChannel::from_u8(0), ChatChannel::General);
    assert_eq!(ChatChannel::from_u8(1), ChatChannel::World);
    assert_eq!(ChatChannel::from_u8(2), ChatChannel::Faction);
    assert_eq!(ChatChannel::from_u8(3), ChatChannel::Party);
    assert_eq!(ChatChannel::from_u8(4), ChatChannel::Whisper);
    assert_eq!(ChatChannel::from_u8(5), ChatChannel::System);
}
