/// Something the viewer asked that the ghost can answer from what it knows
/// of itself, without a model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    /// How it feels.
    Feeling,
    /// What it is doing.
    Doing,
    /// What it is called.
    Name,
    /// Whether it likes the viewer.
    Liking,
    /// How old it is.
    Age,
    /// What it likes doing most.
    Favorite,
    /// What the weather is like outside.
    Weather,
    /// Whether it is tired.
    Tired,
    /// Whether it is hungry.
    Hungry,
    /// Whether it is bored.
    Bored,
    /// Whether it is lonely or missed the viewer.
    Lonely,
}

// Questions in English, as runs of words to find in the message, compared
// lowercased and without apostrophes. More particular questions come first,
// so "are you tired" is not taken for "are you okay".
const ENGLISH: &[(Question, &[&[&str]])] = &[
    (
        Question::Name,
        &[
            &["whats", "your", "name"],
            &["what", "is", "your", "name"],
            &["who", "are", "you"],
            &["your", "name", "is"],
        ],
    ),
    (
        Question::Age,
        &[&["how", "old", "are", "you"], &["how", "old", "r", "u"]],
    ),
    (
        Question::Liking,
        &[
            &["do", "you", "like", "me"],
            &["do", "you", "love", "me"],
            &["are", "we", "friends"],
            &["do", "u", "like", "me"],
            &["do", "u", "love", "me"],
        ],
    ),
    (
        Question::Favorite,
        &[
            &["whats", "your", "favorite"],
            &["whats", "your", "favourite"],
            &["what", "is", "your", "favorite"],
            &["what", "do", "you", "like"],
            &["what", "do", "you", "love"],
            &["what", "do", "you", "enjoy"],
        ],
    ),
    (
        Question::Weather,
        &[
            &["hows", "the", "weather"],
            &["whats", "the", "weather"],
            &["what", "is", "the", "weather"],
            &["how", "is", "the", "weather"],
            &["is", "it", "raining"],
            &["is", "it", "snowing"],
        ],
    ),
    (
        Question::Tired,
        &[
            &["are", "you", "tired"],
            &["are", "you", "sleepy"],
            &["r", "u", "tired"],
            &["you", "tired"],
            &["you", "sleepy"],
        ],
    ),
    (
        Question::Hungry,
        &[
            &["are", "you", "hungry"],
            &["you", "hungry"],
            &["r", "u", "hungry"],
        ],
    ),
    (
        Question::Bored,
        &[
            &["are", "you", "bored"],
            &["you", "bored"],
            &["r", "u", "bored"],
        ],
    ),
    (
        Question::Lonely,
        &[
            &["are", "you", "lonely"],
            &["did", "you", "miss", "me"],
            &["do", "you", "miss", "me"],
            &["you", "lonely"],
            &["miss", "me"],
        ],
    ),
    (
        Question::Doing,
        &[
            &["what", "are", "you", "doing"],
            &["what", "you", "doing"],
            &["whatcha", "doing"],
            &["whatcha", "doin"],
            &["what", "are", "you", "up", "to"],
            &["whats", "up"],
            &["wyd"],
            &["wassup"],
            &["sup"],
        ],
    ),
    (
        Question::Feeling,
        &[
            &["how", "are", "you"],
            &["how", "r", "u"],
            &["how", "do", "you", "feel"],
            &["how", "you", "doing"],
            &["how", "are", "things"],
            &["hows", "it", "going"],
            &["how", "have", "you", "been"],
            &["are", "you", "ok"],
            &["are", "you", "okay"],
            &["are", "you", "alright"],
            &["are", "you", "happy"],
            &["you", "ok"],
            &["you", "okay"],
        ],
    ),
];

// Questions in Korean, as pieces of the message with its spaces taken out,
// and whether the message must end in a question mark, for words that are
// statements just as often ("졸려" is also "I'm sleepy").
const KOREAN: &[(Question, &[&str], bool)] = &[
    (
        Question::Name,
        &[
            "이름이뭐",
            "이름뭐",
            "이름이머",
            "누구야",
            "누구니",
            "누구세요",
        ],
        false,
    ),
    (Question::Age, &["몇살", "나이가어떻게"], false),
    (
        Question::Liking,
        &["나좋아해", "나사랑해", "우리친구"],
        true,
    ),
    (
        Question::Favorite,
        &["뭐좋아해", "뭘좋아해", "제일좋아하는"],
        false,
    ),
    (
        Question::Weather,
        &["날씨어때", "날씨는어때", "비와", "눈와"],
        false,
    ),
    (
        Question::Tired,
        &["졸려", "피곤해", "졸리니", "피곤하니"],
        true,
    ),
    (Question::Hungry, &["배고파", "배고프니"], true),
    (Question::Bored, &["심심해", "심심하니"], true),
    (Question::Lonely, &["외로워", "외롭니", "보고싶었"], true),
    (
        Question::Doing,
        &["뭐해", "뭐하니", "뭐하고있", "뭐하는중", "머해"],
        false,
    ),
    (
        Question::Feeling,
        &[
            "기분어때",
            "기분은어때",
            "잘지냈",
            "잘지내",
            "괜찮아",
            "어떻게지내",
        ],
        false,
    ),
];

/// The question `text` asks, if it is one the ghost can answer by itself.
pub fn ask(text: &str) -> Option<Question> {
    let words: Vec<String> = text
        .to_lowercase()
        .replace(['\'', '’'], "")
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    let english = ENGLISH.iter().find(|(_, runs)| {
        runs.iter().any(|run| {
            words.windows(run.len()).any(|window| {
                window
                    .iter()
                    .zip(run.iter())
                    .all(|(word, part)| word == part)
            })
        })
    });
    if let Some((question, _)) = english {
        return Some(*question);
    }
    let asked = text.trim_end().ends_with(['?', '？']);
    let joined: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    KOREAN
        .iter()
        .find(|(_, pieces, needs_mark)| {
            (asked || !needs_mark) && pieces.iter().any(|piece| joined.contains(piece))
        })
        .map(|(question, _, _)| *question)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everyday_questions_in_english_are_recognized() {
        for (text, question) in [
            ("How are you?", Question::Feeling),
            ("hi! how's it going", Question::Feeling),
            ("are you okay", Question::Feeling),
            ("What are you doing?", Question::Doing),
            ("whatcha doing", Question::Doing),
            ("what's up", Question::Doing),
            ("What's your name?", Question::Name),
            ("who are you", Question::Name),
            ("Do you like me?", Question::Liking),
            ("do you love me", Question::Liking),
            ("How old are you?", Question::Age),
            ("what's your favorite thing", Question::Favorite),
            ("what do you like to do", Question::Favorite),
            ("How's the weather?", Question::Weather),
            ("is it raining", Question::Weather),
            ("are you tired?", Question::Tired),
            ("you sleepy?", Question::Tired),
            ("Are you hungry?", Question::Hungry),
            ("are you bored", Question::Bored),
            ("did you miss me?", Question::Lonely),
        ] {
            assert_eq!(ask(text), Some(question), "{text}");
        }
    }

    #[test]
    fn everyday_questions_in_korean_are_recognized() {
        for (text, question) in [
            ("뭐해?", Question::Doing),
            ("지금 뭐 하고 있어", Question::Doing),
            ("기분 어때?", Question::Feeling),
            ("잘 지냈어?", Question::Feeling),
            ("이름이 뭐야?", Question::Name),
            ("너 누구야", Question::Name),
            ("몇 살이야?", Question::Age),
            ("나 좋아해?", Question::Liking),
            ("뭐 좋아해?", Question::Favorite),
            ("날씨 어때?", Question::Weather),
            ("졸려?", Question::Tired),
            ("배고파?", Question::Hungry),
            ("심심해?", Question::Bored),
            ("보고 싶었어?", Question::Lonely),
        ] {
            assert_eq!(ask(text), Some(question), "{text}");
        }
    }

    #[test]
    fn statements_requests_and_lookalikes_are_not_questions() {
        for text in [
            "hello",
            "good job",
            "fetch the ball",
            "I'm tired",
            "졸려",
            "배고파",
            "사랑해",
            "how about a dance",
            "show me what you can do",
            "you are so cute",
            "",
        ] {
            assert_eq!(ask(text), None, "{text}");
        }
    }

    #[test]
    fn particular_questions_win_over_general_ones() {
        assert_eq!(ask("are you tired? are you okay?"), Some(Question::Tired));
        assert_eq!(ask("how are you, what's your name"), Some(Question::Name));
    }
}
