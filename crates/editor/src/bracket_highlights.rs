use crate::{editor_settings::EditorSettings, Editor, RangeToAnchorExt};
use gpui::{Context, HighlightStyle, Hsla, Window, hsla};
use itertools::Itertools;
use language::CursorShape;
use multi_buffer::ToPoint;
use settings::Settings;
use text::{Bias, OffsetRangeExt, Point};

enum MatchingBracketHighlight {}

struct RainbowBracketHighlight;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BracketRefreshReason {
    BufferEdited,
    ScrollPositionChanged,
    SelectionsChanged,
}

impl Editor {
    // todo! run with a debounce
    pub(crate) fn refresh_bracket_highlights(
        &mut self,
        refresh_reason: BracketRefreshReason,
        window: &mut Window,
        cx: &mut Context<Editor>,
    ) {
        let settings = EditorSettings::get_global(cx);
        let rainbow_settings = &settings.rainbow_brackets;

        if !rainbow_settings.enabled {
            return;
        }

        let get_color_for_depth = |depth: usize| -> Hsla {
            let hue = (rainbow_settings.start_hue + (depth as f32 * rainbow_settings.hue_step)) % 360.0;
            hsla(hue / 360.0, 0.75, 0.6, 1.0)
        };

        let snapshot = self.snapshot(window, cx);
        let multi_buffer_snapshot = &snapshot.buffer_snapshot;

        let multi_buffer_visible_start = snapshot
            .scroll_anchor
            .anchor
            .to_point(multi_buffer_snapshot);

        // todo! deduplicate?
        let multi_buffer_visible_end = multi_buffer_snapshot.clip_point(
            multi_buffer_visible_start
                + Point::new(self.visible_line_count().unwrap_or(40.).ceil() as u32, 0),
            Bias::Left,
        );

        let bracket_matches = multi_buffer_snapshot
            .range_to_buffer_ranges(multi_buffer_visible_start..multi_buffer_visible_end)
            .into_iter()
            .filter_map(|(buffer_snapshot, buffer_range, _)| {
                let buffer_brackets =
                    buffer_snapshot.bracket_ranges(buffer_range.start..buffer_range.end);

                // todo! is there a good way to use the excerpt_id instead?
                let mut excerpt = multi_buffer_snapshot.excerpt_containing(buffer_range.clone())?;

                Some(
                    buffer_brackets
                        .into_iter()
                        .filter_map(|pair| {
                            let buffer_range = pair.open_range.start..pair.close_range.end;
                            if excerpt.contains_buffer_range(buffer_range) {
                                Some((
                                    pair.depth,
                                    excerpt.map_range_from_buffer(pair.open_range),
                                    excerpt.map_range_from_buffer(pair.close_range),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .into_group_map_by(|&(depth, ..)| depth);

        for (depth, bracket_highlights) in dbg!(bracket_matches) {
            let style = HighlightStyle {
                color: Some(get_color_for_depth(depth)),
                ..HighlightStyle::default()
            };

            self.highlight_text_key::<RainbowBracketHighlight>(
                depth,
                bracket_highlights
                    .into_iter()
                    .flat_map(|(_, open, close)| {
                        dbg!((
                            depth,
                            multi_buffer_snapshot.offset_to_point(open.start)
                                ..multi_buffer_snapshot.offset_to_point(open.end),
                            multi_buffer_snapshot.offset_to_point(close.start)
                                ..multi_buffer_snapshot.offset_to_point(close.end),
                        ));
                        [
                            open.to_anchors(&multi_buffer_snapshot),
                            close.to_anchors(&multi_buffer_snapshot),
                        ]
                    })
                    .collect(),
                style,
                cx,
            );
        }

        if dbg!(refresh_reason) == BracketRefreshReason::ScrollPositionChanged {
            return;
        }
        self.clear_background_highlights::<MatchingBracketHighlight>(cx);

        let newest_selection = self.selections.newest::<usize>(cx);
        // Don't highlight brackets if the selection isn't empty
        if !newest_selection.is_empty() {
            return;
        }

        let head = newest_selection.head();
        if head > snapshot.buffer_snapshot.len() {
            log::error!("bug: cursor offset is out of range while refreshing bracket highlights");
            return;
        }

        let mut tail = head;
        if (self.cursor_shape == CursorShape::Block || self.cursor_shape == CursorShape::Hollow)
            && head < snapshot.buffer_snapshot.len()
        {
            if let Some(tail_ch) = snapshot.buffer_snapshot.chars_at(tail).next() {
                tail += tail_ch.len_utf8();
            }
        }

        if let Some((opening_range, closing_range)) = snapshot
            .buffer_snapshot
            .innermost_enclosing_bracket_ranges(head..tail, None)
        {
            self.highlight_background::<MatchingBracketHighlight>(
                &[
                    opening_range.to_anchors(&snapshot.buffer_snapshot),
                    closing_range.to_anchors(&snapshot.buffer_snapshot),
                ],
                |theme| theme.colors().editor_document_highlight_bracket_background,
                cx,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor_tests::init_test, test::editor_lsp_test_context::EditorLspTestContext};
    use indoc::indoc;
    use language::{BracketPair, BracketPairConfig, Language, LanguageConfig, LanguageMatcher};

    #[gpui::test]
    async fn test_matching_bracket_highlights(cx: &mut gpui::TestAppContext) {
        init_test(cx, |_| {});

        let mut cx = EditorLspTestContext::new(
            Language::new(
                LanguageConfig {
                    name: "Rust".into(),
                    matcher: LanguageMatcher {
                        path_suffixes: vec!["rs".to_string()],
                        ..Default::default()
                    },
                    brackets: BracketPairConfig {
                        pairs: vec![
                            BracketPair {
                                start: "{".to_string(),
                                end: "}".to_string(),
                                close: false,
                                surround: false,
                                newline: true,
                            },
                            BracketPair {
                                start: "(".to_string(),
                                end: ")".to_string(),
                                close: false,
                                surround: false,
                                newline: true,
                            },
                        ],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Some(tree_sitter_rust::LANGUAGE.into()),
            )
            .with_brackets_query(indoc! {r#"
                ("{" @open "}" @close)
                ("(" @open ")" @close)
                "#})
            .unwrap(),
            Default::default(),
            cx,
        )
        .await;

        // positioning cursor inside bracket highlights both
        cx.set_state(indoc! {r#"
            pub fn test("Test ˇargument") {
                another_test(1, 2, 3);
            }
        "#});
        cx.assert_editor_background_highlights::<MatchingBracketHighlight>(indoc! {r#"
            pub fn test«(»"Test argument"«)» {
                another_test(1, 2, 3);
            }
        "#});

        cx.set_state(indoc! {r#"
            pub fn test("Test argument") {
                another_test(1, ˇ2, 3);
            }
        "#});
        cx.assert_editor_background_highlights::<MatchingBracketHighlight>(indoc! {r#"
            pub fn test("Test argument") {
                another_test«(»1, 2, 3«)»;
            }
        "#});

        cx.set_state(indoc! {r#"
            pub fn test("Test argument") {
                anotherˇ_test(1, 2, 3);
            }
        "#});
        cx.assert_editor_background_highlights::<MatchingBracketHighlight>(indoc! {r#"
            pub fn test("Test argument") «{»
                another_test(1, 2, 3);
            «}»
        "#});

        // positioning outside of brackets removes highlight
        cx.set_state(indoc! {r#"
            pub fˇn test("Test argument") {
                another_test(1, 2, 3);
            }
        "#});
        cx.assert_editor_background_highlights::<MatchingBracketHighlight>(indoc! {r#"
            pub fn test("Test argument") {
                another_test(1, 2, 3);
            }
        "#});

        // non empty selection dismisses highlight
        cx.set_state(indoc! {r#"
            pub fn test("Te«st argˇ»ument") {
                another_test(1, 2, 3);
            }
        "#});
        cx.assert_editor_background_highlights::<MatchingBracketHighlight>(indoc! {r#"
            pub fn test("Test argument") {
                another_test(1, 2, 3);
            }
        "#});
    }

    #[gpui::test]
    async fn test_rainbow_bracket_colors_differ_by_depth(cx: &mut gpui::TestAppContext) {
        init_test(cx, |settings| {
            settings.rainbow_brackets.enabled = true;
            settings.rainbow_brackets.start_hue = 0.0;
            settings.rainbow_brackets.hue_step = 30.0;
        });

        let get_color_for_depth = |depth: usize| -> Hsla {
            let hue = (0.0 + (depth as f32 * 30.0)) % 360.0;
            hsla(hue / 360.0, 0.75, 0.6, 1.0)
        };

        let color_0 = get_color_for_depth(0);
        let color_1 = get_color_for_depth(1);
        let color_2 = get_color_for_depth(2);

        assert_ne!(color_0, color_1, "Depth 0 and 1 should have different colors");
        assert_ne!(color_1, color_2, "Depth 1 and 2 should have different colors");
        assert_ne!(color_0, color_2, "Depth 0 and 2 should have different colors");
    }

    #[gpui::test]
    async fn test_rainbow_bracket_hue_wraps_at_360(cx: &mut gpui::TestAppContext) {
        init_test(cx, |settings| {
            settings.rainbow_brackets.enabled = true;
            settings.rainbow_brackets.start_hue = 350.0;
            settings.rainbow_brackets.hue_step = 30.0;
        });

        let get_color_for_depth = |depth: usize| -> Hsla {
            let hue = (350.0 + (depth as f32 * 30.0)) % 360.0;
            hsla(hue / 360.0, 0.75, 0.6, 1.0)
        };

        let color_0 = get_color_for_depth(0);
        let color_1 = get_color_for_depth(1);

        assert_eq!(color_0.h, 350.0 / 360.0, "Depth 0 hue should be 350 degrees");
        assert_eq!(color_1.h, 20.0 / 360.0, "Depth 1 hue should wrap to 20 degrees");
    }
}
