#pragma once

// 三个可重绑切换热键（中英 `keybindings.toggle_english_candidates`、全半角
// `toggle_full_width`、标点 `toggle_punctuation`）在宿主侧的认键逻辑。
//
// 写法（"Ctrl+Shift+KeyE" 这类串）的解析只在 Rust 一侧：共享契约是 client-core
// 的 `KeyChord`，偏好文档经 msime_client_toggle_hotkey_chords 给出每个键的主键
// 规范名与修饰键集合，宿主把规范名解成 ToggleChord 后按事件认键——字母主键经
// `ShortcutLetter` 认当前布局打出的字母，非字母主键按 ANSI keyCode；修饰键四旗
// 精确相等才算命中，与各快捷键分支一直以来的掩码语义一致。

namespace msime::mac
{

struct ToggleChord
{
    bool bound;
    unsigned short keyCode; // 非字母主键的 ANSI keyCode；主键是字母时为 0。
    char letter;            // 主键是字母时的小写字母，经 `ShortcutLetter` 认；否则为 0。
    bool control;
    bool option;
    bool shift;
    bool command;
};

constexpr ToggleChord UnboundToggleChord()
{
    return {false, 0, '\0', false, false, false, false};
}

constexpr bool ToggleChordNameEquals(const char *a, const char *b)
{
    while (*a && *a == *b) { ++a; ++b; }
    return *a == *b;
}

namespace detail
{
struct ToggleChordKeyCode { const char *name; unsigned short keyCode; };

// 非字母主键的规范名 → ANSI keyCode。数字行与 `MSIMECandidateDeletionSlot` 的物理
// 数字键、维护快捷键同一套物理约定；具名键是 kVK_ANSI_* 各键。功能键码是
// Carbon 的 kVK_F1–kVK_F12。
constexpr ToggleChordKeyCode kToggleChordKeyCodes[] = {
    {"Digit1", 18}, {"Digit2", 19}, {"Digit3", 20}, {"Digit4", 21}, {"Digit5", 23},
    {"Digit6", 22}, {"Digit7", 26}, {"Digit8", 28}, {"Digit9", 25}, {"Digit0", 29},
    {"F1", 122}, {"F2", 120}, {"F3", 99}, {"F4", 118}, {"F5", 96}, {"F6", 97},
    {"F7", 98}, {"F8", 100}, {"F9", 101}, {"F10", 109}, {"F11", 103}, {"F12", 111},
    {"Space", 49}, {"Semicolon", 41}, {"Equal", 24}, {"Comma", 43}, {"Minus", 27},
    {"Period", 47}, {"Slash", 44}, {"Backquote", 50}, {"BracketLeft", 33},
    {"Backslash", 42}, {"BracketRight", 30}, {"Quote", 39},
};
} // namespace detail

// chord 主键的规范名（client-core key_chord.rs 的 Display 输出）→ 本机认法。
// 认不出的名字给出 unbound；规范名区分大小写，"keye" 不是 "KeyE"。
constexpr ToggleChord ToggleChordForKeyName(const char *name, bool control, bool option,
                                            bool shift, bool command)
{
    if (!name || !*name) return UnboundToggleChord();
    ToggleChord chord = {true, 0, '\0', control, option, shift, command};
    if (name[0] == 'K' && name[1] == 'e' && name[2] == 'y' &&
        name[3] >= 'A' && name[3] <= 'Z' && name[4] == '\0') {
        chord.letter = static_cast<char>(name[3] - 'A' + 'a');
        return chord;
    }
    for (const detail::ToggleChordKeyCode &entry : detail::kToggleChordKeyCodes)
        if (ToggleChordNameEquals(name, entry.name)) {
            chord.keyCode = entry.keyCode;
            return chord;
        }
    return UnboundToggleChord();
}

// 事件是否命中一个 chord：主键按 chord 的认法（keyCode 或布局字母），修饰键
// 四旗精确相等。`letter` 是 `ShortcutLetter` 对这次按键的回答，非字母主键时它不参与。
constexpr bool ToggleChordMatches(ToggleChord chord, unsigned short keyCode, char letter,
                                  bool control, bool option, bool shift, bool command)
{
    return chord.bound && chord.control == control && chord.option == option &&
           chord.shift == shift && chord.command == command &&
           (chord.keyCode ? keyCode == chord.keyCode : letter == chord.letter);
}

// 全半角的两条触发臂：绑定的 chord，以及 macOS 既有的 Option+Shift+H。后者不算在
// chord 里，由 `keybindings.toggle_fullwidth_option_shift_h` 单独门控。
enum class ToggleFullWidthArm { none, chord, optionShiftH };

constexpr ToggleFullWidthArm FullWidthToggleArmFor(ToggleChord chord, unsigned short keyCode,
    char letter, bool control, bool option, bool shift, bool command)
{
    if (ToggleChordMatches(chord, keyCode, letter, control, option, shift, command))
        return ToggleFullWidthArm::chord;
    if (letter == 'h' && option && shift && !control && !command)
        return ToggleFullWidthArm::optionShiftH;
    return ToggleFullWidthArm::none;
}

// 出厂绑定：client-core `TOGGLE_*_DEFAULT`（preferences.rs）的解码形，是共享文档
// 还没载入、或 keybindings 里没有这个键时的认法。改出厂值要两边一起改。
constexpr ToggleChord FactoryEnglishCandidatesToggleChord()
{
    return ToggleChordForKeyName("KeyE", true, false, true, false);
}
constexpr ToggleChord FactoryFullWidthToggleChord()
{
    return ToggleChordForKeyName("Space", true, false, true, false);
}
constexpr ToggleChord FactoryPunctuationToggleChord()
{
    return ToggleChordForKeyName("Period", true, false, false, false);
}

} // namespace msime::mac
