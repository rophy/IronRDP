use ironrdp_pdu::rdp::client_info::ClientInfoFlags;
use ironrdp_server::audio_playback_wanted;

#[test]
fn audio_playback_follows_client_info_flags() {
    assert!(audio_playback_wanted(ClientInfoFlags::empty()));
    assert!(!audio_playback_wanted(ClientInfoFlags::NO_AUDIO_PLAYBACK));
    assert!(!audio_playback_wanted(ClientInfoFlags::REMOTE_CONSOLE_AUDIO));
    assert!(!audio_playback_wanted(
        ClientInfoFlags::NO_AUDIO_PLAYBACK | ClientInfoFlags::REMOTE_CONSOLE_AUDIO
    ));
}
