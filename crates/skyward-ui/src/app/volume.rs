//! The volume panel: the card the volume keys put up when they turn the
//! volume rather than the light, and the finger working it.

use super::*;

impl App {
    /// Draw the panel over the page, if it is up. The page under it is left
    /// whole — no dimming and no blur, unlike the power menu's card.
    pub(super) fn compose_volume_panel(&mut self, out: &mut Canvas, f: &Fonts) {
        if self.volume_bar.is_none() {
            return;
        }
        let t = self.theme();
        let l = self.state.lang;
        widgets::volume_panel(
            out,
            &t,
            f,
            l.among("系统（铃声和音效）", "系統（鈴聲和音效）", "System (ringtone and sounds)"),
            l.among("媒体（应用和音乐）", "媒體（應用和音樂）", "Media (apps and music)"),
            l.among("静音", "靜音", "Mute"),
            self.state.system_volume,
            self.state.media_volume,
            self.state.volume_media,
            self.state.muted,
        );
    }

    /// A finger landing on the panel: a track takes it as any slider does,
    /// moving to where it landed and going on with the finger; the switch's
    /// row turns the phone silent; a tap anywhere else puts the panel away,
    /// and the page under it does not get the tap.
    pub(super) fn volume_panel_touch(&mut self, x: f32, y: f32, now: u64) -> Vec<Effect> {
        self.stir(now);
        let t = self.theme();
        let g = widgets::volume_panel_geom(&t);
        for (which, track) in [(Slider::SystemVolume, g.system), (Slider::MediaVolume, g.media)] {
            if track.contains(x, y) {
                self.set_slider(&t, which, track, x);
                self.volume_bar = Some(now + VOLUME_BAR_MS);
                self.drag = Drag::Panel(which, track);
                return Vec::new();
            }
        }
        if g.mute.contains(x, y) {
            self.volume_bar = Some(now + VOLUME_BAR_MS);
            return self.act(Action::Toggle(Switch::Mute), now);
        }
        self.volume_bar = None;
        self.dirty = true;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;

    /// Fastboot is worked with the keys whatever the setting says: a volume
    /// key moves the light over its choices, never puts the volume card up
    /// over them.
    #[test]
    fn the_fastboot_page_takes_the_keys_and_shows_no_panel() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Fastboot, W, H, PPD);
        assert!(!app.state.volume_keys, "the setting is off");
        app.frame(&f, &i);
        assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(super::super::reboot::FASTBOOT_CHOICES[0].1)));
        press(&mut app, Key::VolumeDown, 0);
        assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(super::super::reboot::FASTBOOT_CHOICES[1].1)), "the key moved the light");
        assert!(app.volume_bar.is_none(), "and no volume panel came up");
        assert_eq!(app.state.media_volume, 8, "nor did the volume turn");
    }

    /// The panel stands in the middle of the screen, a track for each
    /// volume and the switch's row under them.
    #[test]
    fn the_panel_stands_in_the_middle_with_a_track_for_each_volume() {
        let t = Theme::light(W, H, PPD);
        let g = widgets::volume_panel_geom(&t);
        let (mid, card_mid) = (H as f32 / 2.0, g.card.y + g.card.h / 2.0);
        assert!((mid - card_mid).abs() < 1.0, "the card is centred: {card_mid} of {mid}");
        assert_eq!((g.card.x, g.card.w), (t.m.margin, t.m.w as f32 - t.m.margin * 2.0));
        assert!(g.system.y < g.media.y, "the system volume stands above the media one");
        assert!(g.media.y + g.media.h < g.mute.y, "the switch's row is under both");
        assert!(g.mute.y + g.mute.h <= g.card.y + g.card.h, "and the row is on the card");
    }

    /// The panel is up for a while after the last volume key, and a finger
    /// on its media track moves the media volume at once, with the media
    /// track in hand thereafter.
    #[test]
    fn a_finger_on_a_track_sets_that_volume() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        app.frame(&f, &i);
        app.key(Key::VolumeUp, 1000);
        app.key_up(Key::VolumeUp, 1000);
        assert!(app.volume_bar.is_some(), "the key puts the panel up");

        let t = app.theme();
        let g = widgets::volume_panel_geom(&t);
        app.touch(Touch::Down(g.media.x + g.media.w * 0.2, g.media.y + g.media.h / 2.0), &f, 1100);
        assert_eq!(app.state.media_volume, 3, "a fifth of the span, in its steps");
        assert!(app.state.volume_media, "the media track is the one in hand");
        app.touch(Touch::Up, &f, 1150);

        app.touch(Touch::Down(g.system.x + g.system.w * 0.8, g.system.y + g.system.h / 2.0), &f, 1200);
        assert_eq!(app.state.system_volume, 12);
        assert!(!app.state.volume_media, "and the system track now is");
        app.touch(Touch::Up, &f, 1250);
    }

    /// Dragging along a track goes on moving it, and each move keeps the
    /// panel up.
    #[test]
    fn a_drag_along_a_track_moves_it_and_keeps_the_panel_up() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        app.state.media_volume = 5;
        app.frame(&f, &i);
        app.key(Key::VolumeUp, 1000);
        app.key_up(Key::VolumeUp, 1000);

        let t = app.theme();
        let g = widgets::volume_panel_geom(&t);
        let cy = g.media.y + g.media.h / 2.0;
        app.touch(Touch::Down(g.media.x + 2.0, cy), &f, 1100);
        assert_eq!(app.state.media_volume, 0);
        app.touch(Touch::Move(g.media.x + g.media.w, cy), &f, 2000);
        assert_eq!(app.state.media_volume, 15, "dragged to the far end");
        assert!(app.volume_bar.is_some_and(|m| m > 2000), "the move keeps it up past where it would have gone");
        app.touch(Touch::Up, &f, 2100);
    }

    /// The switch under the tracks turns the phone silent, and the panel
    /// stays up for the thumb that did it.
    #[test]
    fn the_switch_turns_the_phone_silent() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        app.frame(&f, &i);
        app.key(Key::VolumeUp, 1000);
        app.key_up(Key::VolumeUp, 1000);

        let t = app.theme();
        let g = widgets::volume_panel_geom(&t);
        app.touch(Touch::Down(g.mute.x + g.mute.w - t.m.inner, g.mute.y + g.mute.h / 2.0), &f, 1100);
        assert!(app.state.muted, "the switch goes on");
        assert!(app.volume_bar.is_some(), "and the panel stays while it is worked");
    }

    /// A tap off the panel puts it away without reaching the page under it.
    #[test]
    fn a_tap_off_the_panel_puts_it_away() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        app.frame(&f, &i);
        app.key(Key::VolumeUp, 1000);
        app.key_up(Key::VolumeUp, 1000);

        let t = app.theme();
        let g = widgets::volume_panel_geom(&t);
        let desk = app.screen();
        app.touch(Touch::Down(g.card.x / 2.0, g.card.y / 2.0), &f, 1100);
        assert!(app.volume_bar.is_none(), "the panel goes");
        assert_eq!(app.screen(), desk, "and the page under it is not touched");
        app.touch(Touch::Up, &f, 1150);
    }

    /// The volume keys go on turning the media volume with the panel up,
    /// and leave the phone sounding when it was silent.
    #[test]
    fn the_keys_turn_the_media_volume_and_leave_it_sounding() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        app.state.media_volume = 15;
        app.state.muted = true;
        app.frame(&f, &i);
        app.key(Key::VolumeDown, 1000);
        app.key_up(Key::VolumeDown, 1000);
        assert_eq!(app.state.media_volume, 14);
        assert!(!app.state.muted, "moving the volume leaves the phone sounding");
        assert!(app.state.volume_media);
    }

    /// The panel is not put up while the volume keys move the light, and
    /// not over the lock either.
    #[test]
    fn the_panel_waits_for_the_keys_to_be_turned_to_the_volume() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = true;
        app.frame(&f, &i);
        app.key(Key::VolumeUp, 1000);
        app.key_up(Key::VolumeUp, 1000);
        assert!(app.volume_bar.is_none(), "the keys move the light instead");

        app.state.volume_keys = false;
        app.lock_now();
        let lum = app.state.media_volume;
        app.key(Key::VolumeUp, 2000);
        app.tick(2000 + CHORD_MS);
        app.key_up(Key::VolumeUp, 2000);
        assert!(app.volume_bar.is_none(), "and the lock keeps the panel away");
        assert_eq!(app.state.media_volume, lum);
    }

    /// The power menu puts the panel away: the card it draws stands over a
    /// blurred page, and a second card would be left in the blur.
    #[test]
    fn the_power_menu_puts_the_panel_away() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        app.frame(&f, &i);
        app.key(Key::VolumeUp, 1000);
        app.key_up(Key::VolumeUp, 1000);
        assert!(app.volume_bar.is_some());
        app.show_power_menu(1200);
        assert!(app.volume_bar.is_none(), "the panel gives the card the screen");
        let _ = i;
    }
}
