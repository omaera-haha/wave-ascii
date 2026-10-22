use crate::terminal::Key;

pub const LINE_LIMIT: usize = 12;
pub const CHARACTER_LIMIT: usize = 600;

/// A small page editor. Cursor movement follows explicit lines, independent
/// of the terminal width; wrapping belongs to the panel that displays it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    text: String,
    cursor: usize,
    column: Option<usize>,
}

impl Draft {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub fn insert(&mut self, text: &str) {
        let mut length = self.text.chars().count();
        let mut lines = self.text.matches('\n').count() + 1;
        let mut previous = None;
        for character in text.chars() {
            // A pasted CRLF is one break; lone CR also starts a line.
            if character == '\n' && previous == Some('\r') {
                previous = Some(character);
                continue;
            }
            previous = Some(character);
            let character = if character == '\r' { '\n' } else { character };
            if character.is_control() && character != '\n' {
                continue;
            }
            if length >= CHARACTER_LIMIT || (character == '\n' && lines >= LINE_LIMIT) {
                break;
            }
            let index = self.byte_index(self.cursor);
            self.text.insert(index, character);
            self.cursor += 1;
            length += 1;
            lines += usize::from(character == '\n');
        }
        self.column = None;
    }

    pub fn edit(&mut self, key: &Key) {
        let length = self.text.chars().count();
        let before: String = self.text.chars().take(self.cursor).collect();
        let start = before
            .rfind('\n')
            .map_or(0, |index| before[..=index].chars().count());
        let end = self
            .text
            .chars()
            .skip(self.cursor)
            .position(|character| character == '\n')
            .map_or(length, |offset| self.cursor + offset);
        match key {
            Key::Character(character) => self.insert(&character.to_string()),
            Key::Enter => self.insert("\n"),
            Key::Left => self.cursor = self.cursor.saturating_sub(1),
            Key::Right => self.cursor = (self.cursor + 1).min(length),
            Key::Home => self.cursor = start,
            Key::End => self.cursor = end,
            Key::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.byte_index(self.cursor));
            }
            Key::Delete if self.cursor < length => {
                self.text.remove(self.byte_index(self.cursor));
            }
            Key::Up | Key::Down => {
                let column = *self.column.get_or_insert(self.cursor - start);
                if *key == Key::Up && start > 0 {
                    let previous = self.text.chars().take(start - 1).collect::<String>();
                    let previous_start = previous
                        .rfind('\n')
                        .map_or(0, |index| previous[..=index].chars().count());
                    self.cursor = previous_start + column.min(start - 1 - previous_start);
                } else if *key == Key::Down && end < length {
                    let next_length = self
                        .text
                        .chars()
                        .skip(end + 1)
                        .take_while(|character| *character != '\n')
                        .count();
                    self.cursor = end + 1 + column.min(next_length);
                }
                return;
            }
            _ => {}
        }
        self.column = None;
    }

    fn byte_index(&self, character: usize) -> usize {
        self.text
            .char_indices()
            .nth(character)
            .map_or(self.text.len(), |(index, _)| index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_inserts_and_deletes_unicode_and_joins_explicit_lines() {
        let mut draft = Draft::default();
        draft.insert("안녕\nfriend");
        draft.edit(&Key::Home);
        draft.edit(&Key::Backspace);
        assert_eq!(draft.text(), "안녕friend");
        draft.edit(&Key::Left);
        draft.edit(&Key::Delete);
        draft.insert("뇽");
        assert_eq!(draft.text(), "안뇽friend");
        draft.edit(&Key::End);
        draft.edit(&Key::Enter);
        assert_eq!(draft.text(), "안뇽friend\n");
    }

    #[test]
    fn vertical_movement_remembers_the_logical_column_across_short_lines() {
        let mut draft = Draft::default();
        draft.insert("abcdef\n한\n가나다라마바");
        draft.edit(&Key::Up);
        assert_eq!(draft.cursor(), 8);
        draft.edit(&Key::Up);
        assert_eq!(draft.cursor(), 6);
        draft.edit(&Key::Home);
        draft.edit(&Key::Down);
        assert_eq!(draft.cursor(), 7);
        draft.edit(&Key::Right);
        draft.edit(&Key::Down);
        assert_eq!(draft.cursor(), 10);
    }

    #[test]
    fn paste_keeps_line_breaks_and_drops_other_controls_with_bounded_input() {
        let mut draft = Draft::default();
        draft.insert("a\r\nb\rc\t\u{1b}\0");
        assert_eq!(draft.text(), "a\nb\nc");
        draft.insert(&"\n".repeat(20));
        assert_eq!(draft.text().lines().count(), 11);
        assert_eq!(draft.text().matches('\n').count(), 11);
        draft.insert(&"한".repeat(700));
        assert_eq!(draft.text().chars().count(), CHARACTER_LIMIT);
        draft.edit(&Key::Character('q'));
        assert_eq!(draft.text().chars().count(), CHARACTER_LIMIT);
    }
}
