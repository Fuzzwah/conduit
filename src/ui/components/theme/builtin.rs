//! Built-in themes embedded in the binary.
//!
//! These themes are always available regardless of VS Code installation.

use ratatui::style::Color;

use super::types::Theme;

/// Get all built-in themes.
pub fn builtin_themes() -> Vec<(&'static str, Theme)> {
    vec![
        ("default-dark", Theme::default_dark()),
        ("default-light", Theme::default_light()),
        ("catppuccin-mocha", catppuccin_mocha()),
        ("catppuccin-latte", catppuccin_latte()),
        ("catppuccin-frappe", catppuccin_frappe()),
        ("catppuccin-macchiato", catppuccin_macchiato()),
        ("tokyo-night", tokyo_night()),
        ("tokyo-night-moon", tokyo_night_moon()),
        ("tokyo-night-storm", tokyo_night_storm()),
        ("tokyo-night-day", tokyo_night_day()),
        ("dracula", dracula()),
        ("nord", nord()),
        ("rose-pine", rose_pine()),
        ("rose-pine-moon", rose_pine_moon()),
        ("rose-pine-dawn", rose_pine_dawn()),
        ("solarized-dark", solarized_dark()),
        ("solarized-light", solarized_light()),
        ("gruvbox-dark", gruvbox_dark()),
        ("gruvbox-light", gruvbox_light()),
        ("monokai", monokai()),
        ("monokai-pro", monokai_pro()),
        ("monokai-vivid", monokai_vivid()),
        ("tomorrow-night", tomorrow_night()),
        ("tomorrow-night-eighties", tomorrow_night_eighties()),
        ("tomorrow-night-blue", tomorrow_night_blue()),
        ("tomorrow-night-bright", tomorrow_night_bright()),
        ("atom-one-dark", atom_one_dark()),
        ("one-half-dark", one_half_dark()),
        ("github-dark", github_dark()),
        ("github-light", github_light()),
        ("jetbrains-darcula", jetbrains_darcula()),
        ("vscode-dark", vscode_dark()),
        ("vscode-dark-modern", vscode_dark_modern()),
        ("shades-of-purple", shades_of_purple()),
        ("night-owl", night_owl()),
        ("synthwave", synthwave()),
    ]
}

/// Get a built-in theme by name.
pub fn get_builtin(name: &str) -> Option<Theme> {
    if name == "default" {
        return Some(Theme::default_dark());
    }
    builtin_themes()
        .into_iter()
        .find(|(k, _)| *k == name)
        .map(|(_, t)| t)
}

/// Catppuccin Mocha theme.
pub fn catppuccin_mocha() -> Theme {
    // Catppuccin Mocha palette
    // https://github.com/catppuccin/catppuccin
    let crust = Color::Rgb(17, 17, 27);
    let mantle = Color::Rgb(24, 24, 37);
    let base = Color::Rgb(30, 30, 46);
    let surface0 = Color::Rgb(49, 50, 68);
    let surface1 = Color::Rgb(69, 71, 90);
    let surface2 = Color::Rgb(88, 91, 112);
    let overlay0 = Color::Rgb(108, 112, 134);
    let overlay1 = Color::Rgb(127, 132, 156);
    let _overlay2 = Color::Rgb(147, 153, 178);
    let subtext0 = Color::Rgb(166, 173, 200);
    let subtext1 = Color::Rgb(186, 194, 222);
    let text = Color::Rgb(205, 214, 244);

    let blue = Color::Rgb(137, 180, 250);
    let mauve = Color::Rgb(203, 166, 247);
    let green = Color::Rgb(166, 227, 161);
    let yellow = Color::Rgb(249, 226, 175);
    let red = Color::Rgb(243, 139, 168);
    let sky = Color::Rgb(137, 220, 235);

    Theme {
        name: "Catppuccin Mocha".to_string(),
        is_light: false,

        bg_terminal: crust,
        bg_base: base,
        bg_surface: surface0,
        bg_elevated: surface1,
        bg_highlight: surface2,
        markdown_code_bg: mantle,
        markdown_inline_code_bg: surface0,

        text_bright: text,
        text_primary: subtext1,
        text_secondary: subtext0,
        text_muted: overlay1,
        text_faint: overlay0,

        accent_primary: blue,
        accent_secondary: mauve,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: sky,
        agent_codex: mauve,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: mauve,
        pr_closed_bg: red,
        pr_draft_bg: overlay1,
        pr_unknown_bg: surface2,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(120, 160, 220),
        spinner_trail_2: Color::Rgb(100, 140, 190),
        spinner_trail_3: Color::Rgb(80, 115, 160),
        spinner_trail_4: Color::Rgb(60, 90, 130),
        spinner_trail_5: Color::Rgb(45, 70, 100),
        spinner_inactive: overlay1,

        border_default: surface1,
        border_focused: blue,
        border_dimmed: surface0,

        shine_edge: overlay1,
        shine_mid: subtext0,
        shine_center: subtext1,
        shine_peak: text,

        tool_block_bg: mantle,
        tool_comment: overlay1,
        tool_command: subtext1,
        tool_output: subtext0,
        diff_add: green,
        diff_remove: red,
    }
}

/// Catppuccin Latte theme (light).
pub fn catppuccin_latte() -> Theme {
    // Catppuccin Latte palette (light)
    let base = Color::Rgb(239, 241, 245);
    let mantle = Color::Rgb(230, 233, 239);
    let crust = Color::Rgb(220, 224, 232);
    let surface0 = Color::Rgb(204, 208, 218);
    let surface1 = Color::Rgb(188, 192, 204);
    let surface2 = Color::Rgb(172, 176, 190);
    let overlay0 = Color::Rgb(156, 160, 176);
    let overlay1 = Color::Rgb(140, 143, 161);
    let _overlay2 = Color::Rgb(124, 127, 147);
    let subtext0 = Color::Rgb(108, 111, 133);
    let subtext1 = Color::Rgb(92, 95, 119);
    let text = Color::Rgb(76, 79, 105);

    let blue = Color::Rgb(30, 102, 245);
    let mauve = Color::Rgb(136, 57, 239);
    let green = Color::Rgb(64, 160, 43);
    let yellow = Color::Rgb(223, 142, 29);
    let red = Color::Rgb(210, 15, 57);
    let sky = Color::Rgb(4, 165, 229);

    Theme {
        name: "Catppuccin Latte".to_string(),
        is_light: true,

        bg_terminal: base,
        bg_base: base,
        bg_surface: mantle,
        bg_elevated: crust,
        bg_highlight: surface0,
        markdown_code_bg: mantle,
        markdown_inline_code_bg: crust,

        text_bright: text,
        text_primary: subtext1,
        text_secondary: subtext0,
        text_muted: overlay1,
        text_faint: overlay0,

        accent_primary: blue,
        accent_secondary: mauve,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: sky,
        agent_codex: mauve,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: mauve,
        pr_closed_bg: red,
        pr_draft_bg: overlay1,
        pr_unknown_bg: surface2,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(60, 130, 240),
        spinner_trail_2: Color::Rgb(90, 150, 235),
        spinner_trail_3: Color::Rgb(120, 170, 230),
        spinner_trail_4: Color::Rgb(150, 190, 225),
        spinner_trail_5: Color::Rgb(180, 205, 220),
        spinner_inactive: overlay1,

        border_default: surface1,
        border_focused: blue,
        border_dimmed: surface0,

        shine_edge: overlay0,
        shine_mid: subtext0,
        shine_center: subtext1,
        shine_peak: text,

        tool_block_bg: mantle,
        tool_comment: overlay1,
        tool_command: subtext1,
        tool_output: subtext0,
        diff_add: green,
        diff_remove: red,
    }
}

/// Tokyo Night theme.
pub fn tokyo_night() -> Theme {
    // Tokyo Night palette
    let bg = Color::Rgb(26, 27, 38);
    let bg_dark = Color::Rgb(22, 22, 30);
    let bg_highlight = Color::Rgb(41, 46, 66);
    let terminal_black = Color::Rgb(65, 72, 104);
    let fg = Color::Rgb(192, 202, 245);
    let fg_dark = Color::Rgb(169, 177, 214);
    let fg_gutter = Color::Rgb(59, 66, 97);
    let dark3 = Color::Rgb(68, 75, 106);
    let comment = Color::Rgb(86, 95, 137);
    let dark5 = Color::Rgb(115, 125, 174);
    let blue0 = Color::Rgb(61, 89, 161);
    let blue = Color::Rgb(122, 162, 247);
    let cyan = Color::Rgb(125, 207, 255);
    let magenta = Color::Rgb(187, 154, 247);
    let green = Color::Rgb(158, 206, 106);
    let yellow = Color::Rgb(224, 175, 104);
    let red = Color::Rgb(247, 118, 142);

    Theme {
        name: "Tokyo Night".to_string(),
        is_light: false,

        bg_terminal: bg_dark,
        bg_base: bg,
        bg_surface: Color::Rgb(36, 40, 59),
        bg_elevated: bg_highlight,
        bg_highlight: Color::Rgb(51, 59, 91),
        markdown_code_bg: bg_dark,
        markdown_inline_code_bg: Color::Rgb(36, 40, 59),

        text_bright: fg,
        text_primary: fg_dark,
        text_secondary: dark5,
        text_muted: comment,
        text_faint: fg_gutter,

        accent_primary: blue,
        accent_secondary: magenta,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: magenta,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: magenta,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: dark3,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(105, 145, 220),
        spinner_trail_2: Color::Rgb(85, 125, 195),
        spinner_trail_3: Color::Rgb(70, 105, 170),
        spinner_trail_4: Color::Rgb(55, 85, 145),
        spinner_trail_5: Color::Rgb(45, 70, 120),
        spinner_inactive: comment,

        border_default: terminal_black,
        border_focused: blue0,
        border_dimmed: dark3,

        shine_edge: comment,
        shine_mid: dark5,
        shine_center: fg_dark,
        shine_peak: fg,

        tool_block_bg: bg_dark,
        tool_comment: comment,
        tool_command: fg_dark,
        tool_output: dark5,
        diff_add: green,
        diff_remove: red,
    }
}

/// Dracula theme.
pub fn dracula() -> Theme {
    // Dracula palette
    let background = Color::Rgb(40, 42, 54);
    let current_line = Color::Rgb(68, 71, 90);
    let foreground = Color::Rgb(248, 248, 242);
    let comment = Color::Rgb(98, 114, 164);
    let cyan = Color::Rgb(139, 233, 253);
    let green = Color::Rgb(80, 250, 123);
    let orange = Color::Rgb(255, 184, 108);
    let pink = Color::Rgb(255, 121, 198);
    let purple = Color::Rgb(189, 147, 249);
    let red = Color::Rgb(255, 85, 85);
    let _yellow = Color::Rgb(241, 250, 140); // Available if needed

    Theme {
        name: "Dracula".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(30, 32, 44),
        bg_base: background,
        bg_surface: current_line,
        bg_elevated: Color::Rgb(78, 81, 100),
        bg_highlight: Color::Rgb(88, 91, 110),
        markdown_code_bg: Color::Rgb(35, 37, 49),
        markdown_inline_code_bg: current_line,

        text_bright: foreground,
        text_primary: Color::Rgb(235, 235, 230),
        text_secondary: Color::Rgb(180, 185, 200),
        text_muted: comment,
        text_faint: Color::Rgb(75, 85, 120),

        accent_primary: purple,
        accent_secondary: pink,
        accent_success: green,
        accent_warning: orange,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: pink,
        agent_opencode: purple,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: current_line,

        spinner_active: purple,
        spinner_trail_1: Color::Rgb(165, 130, 225),
        spinner_trail_2: Color::Rgb(140, 115, 200),
        spinner_trail_3: Color::Rgb(115, 100, 175),
        spinner_trail_4: Color::Rgb(90, 85, 150),
        spinner_trail_5: Color::Rgb(70, 75, 125),
        spinner_inactive: comment,

        border_default: comment,
        border_focused: purple,
        border_dimmed: Color::Rgb(60, 65, 95),

        shine_edge: comment,
        shine_mid: Color::Rgb(150, 155, 180),
        shine_center: Color::Rgb(200, 200, 210),
        shine_peak: foreground,

        tool_block_bg: Color::Rgb(35, 37, 49),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(180, 185, 200),
        diff_add: green,
        diff_remove: red,
    }
}

/// Catppuccin Frappé theme.
pub fn catppuccin_frappe() -> Theme {
    let crust = Color::Rgb(35, 38, 52);
    let mantle = Color::Rgb(41, 44, 60);
    let base = Color::Rgb(48, 52, 70);
    let surface0 = Color::Rgb(65, 69, 89);
    let surface1 = Color::Rgb(81, 87, 109);
    let surface2 = Color::Rgb(98, 104, 128);
    let overlay0 = Color::Rgb(115, 121, 148);
    let overlay1 = Color::Rgb(131, 139, 167);
    let subtext0 = Color::Rgb(165, 173, 206);
    let subtext1 = Color::Rgb(181, 191, 226);
    let text = Color::Rgb(198, 208, 245);
    let blue = Color::Rgb(140, 170, 238);
    let mauve = Color::Rgb(202, 158, 230);
    let green = Color::Rgb(166, 209, 137);
    let yellow = Color::Rgb(229, 200, 144);
    let red = Color::Rgb(231, 130, 132);
    let sky = Color::Rgb(153, 209, 219);

    Theme {
        name: "Catppuccin Frappé".to_string(),
        is_light: false,

        bg_terminal: crust,
        bg_base: base,
        bg_surface: surface0,
        bg_elevated: surface1,
        bg_highlight: surface2,
        markdown_code_bg: mantle,
        markdown_inline_code_bg: surface0,

        text_bright: text,
        text_primary: subtext1,
        text_secondary: subtext0,
        text_muted: overlay1,
        text_faint: overlay0,

        accent_primary: blue,
        accent_secondary: mauve,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: sky,
        agent_codex: mauve,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: mauve,
        pr_closed_bg: red,
        pr_draft_bg: overlay1,
        pr_unknown_bg: surface2,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(120, 150, 210),
        spinner_trail_2: Color::Rgb(100, 130, 185),
        spinner_trail_3: Color::Rgb(80, 110, 160),
        spinner_trail_4: Color::Rgb(60, 88, 130),
        spinner_trail_5: Color::Rgb(45, 65, 100),
        spinner_inactive: overlay1,

        border_default: surface1,
        border_focused: blue,
        border_dimmed: surface0,

        shine_edge: overlay1,
        shine_mid: subtext0,
        shine_center: subtext1,
        shine_peak: text,

        tool_block_bg: mantle,
        tool_comment: overlay1,
        tool_command: subtext1,
        tool_output: subtext0,
        diff_add: green,
        diff_remove: red,
    }
}

/// Catppuccin Macchiato theme.
pub fn catppuccin_macchiato() -> Theme {
    let crust = Color::Rgb(24, 25, 38);
    let mantle = Color::Rgb(30, 32, 48);
    let base = Color::Rgb(36, 39, 58);
    let surface0 = Color::Rgb(54, 58, 79);
    let surface1 = Color::Rgb(73, 77, 100);
    let surface2 = Color::Rgb(91, 96, 120);
    let overlay0 = Color::Rgb(110, 115, 141);
    let overlay1 = Color::Rgb(128, 135, 162);
    let subtext0 = Color::Rgb(165, 173, 203);
    let subtext1 = Color::Rgb(184, 192, 224);
    let text = Color::Rgb(202, 211, 245);
    let blue = Color::Rgb(138, 173, 244);
    let mauve = Color::Rgb(198, 160, 246);
    let green = Color::Rgb(166, 218, 149);
    let yellow = Color::Rgb(238, 212, 159);
    let red = Color::Rgb(237, 135, 150);
    let sky = Color::Rgb(145, 215, 227);

    Theme {
        name: "Catppuccin Macchiato".to_string(),
        is_light: false,

        bg_terminal: crust,
        bg_base: base,
        bg_surface: surface0,
        bg_elevated: surface1,
        bg_highlight: surface2,
        markdown_code_bg: mantle,
        markdown_inline_code_bg: surface0,

        text_bright: text,
        text_primary: subtext1,
        text_secondary: subtext0,
        text_muted: overlay1,
        text_faint: overlay0,

        accent_primary: blue,
        accent_secondary: mauve,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: sky,
        agent_codex: mauve,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: mauve,
        pr_closed_bg: red,
        pr_draft_bg: overlay1,
        pr_unknown_bg: surface2,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(118, 153, 220),
        spinner_trail_2: Color::Rgb(98, 133, 195),
        spinner_trail_3: Color::Rgb(78, 110, 168),
        spinner_trail_4: Color::Rgb(60, 88, 140),
        spinner_trail_5: Color::Rgb(44, 66, 110),
        spinner_inactive: overlay1,

        border_default: surface1,
        border_focused: blue,
        border_dimmed: surface0,

        shine_edge: overlay1,
        shine_mid: subtext0,
        shine_center: subtext1,
        shine_peak: text,

        tool_block_bg: mantle,
        tool_comment: overlay1,
        tool_command: subtext1,
        tool_output: subtext0,
        diff_add: green,
        diff_remove: red,
    }
}

/// Nord theme.
pub fn nord() -> Theme {
    // Polar Night
    let nord0 = Color::Rgb(46, 52, 64);
    let nord1 = Color::Rgb(59, 66, 82);
    let nord2 = Color::Rgb(67, 76, 94);
    let nord3 = Color::Rgb(76, 86, 106);
    // Snow Storm
    let nord4 = Color::Rgb(216, 222, 233);
    let nord5 = Color::Rgb(229, 233, 240);
    let nord6 = Color::Rgb(236, 239, 244);
    // Frost
    let nord7 = Color::Rgb(143, 188, 187);
    let nord8 = Color::Rgb(136, 192, 208);
    let nord9 = Color::Rgb(129, 161, 193);
    let nord10 = Color::Rgb(94, 129, 172);
    // Aurora
    let nord11 = Color::Rgb(191, 97, 106);
    let nord13 = Color::Rgb(235, 203, 139);
    let nord14 = Color::Rgb(163, 190, 140);
    let nord15 = Color::Rgb(180, 142, 173);

    Theme {
        name: "Nord".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(36, 42, 54),
        bg_base: nord0,
        bg_surface: nord1,
        bg_elevated: nord2,
        bg_highlight: nord3,
        markdown_code_bg: Color::Rgb(36, 42, 54),
        markdown_inline_code_bg: nord1,

        text_bright: nord6,
        text_primary: nord5,
        text_secondary: nord4,
        text_muted: nord3,
        text_faint: Color::Rgb(67, 76, 94),

        accent_primary: nord8,
        accent_secondary: nord15,
        accent_success: nord14,
        accent_warning: nord13,
        accent_error: nord11,

        agent_claude: nord7,
        agent_codex: nord15,
        agent_opencode: nord9,

        pr_open_bg: nord14,
        pr_merged_bg: nord15,
        pr_closed_bg: nord11,
        pr_draft_bg: nord3,
        pr_unknown_bg: nord2,

        spinner_active: nord8,
        spinner_trail_1: Color::Rgb(115, 170, 190),
        spinner_trail_2: Color::Rgb(95, 148, 168),
        spinner_trail_3: Color::Rgb(76, 125, 145),
        spinner_trail_4: Color::Rgb(60, 103, 122),
        spinner_trail_5: Color::Rgb(50, 80, 110),
        spinner_inactive: nord3,

        border_default: nord1,
        border_focused: nord10,
        border_dimmed: Color::Rgb(54, 61, 75),

        shine_edge: nord3,
        shine_mid: nord4,
        shine_center: nord5,
        shine_peak: nord6,

        tool_block_bg: Color::Rgb(36, 42, 54),
        tool_comment: nord3,
        tool_command: nord5,
        tool_output: nord4,
        diff_add: nord14,
        diff_remove: nord11,
    }
}

/// Tokyo Night Moon theme.
pub fn tokyo_night_moon() -> Theme {
    let bg = Color::Rgb(34, 36, 54);
    let bg_dark = Color::Rgb(27, 29, 44);
    let bg_highlight = Color::Rgb(44, 50, 75);
    let fg = Color::Rgb(195, 205, 248);
    let fg_dark = Color::Rgb(172, 180, 218);
    let fg_gutter = Color::Rgb(62, 68, 100);
    let dark3 = Color::Rgb(70, 77, 110);
    let comment = Color::Rgb(90, 100, 145);
    let dark5 = Color::Rgb(118, 128, 178);
    let blue = Color::Rgb(130, 170, 255);
    let cyan = Color::Rgb(134, 225, 252);
    let magenta = Color::Rgb(195, 160, 255);
    let green = Color::Rgb(180, 220, 130);
    let yellow = Color::Rgb(255, 205, 110);
    let red = Color::Rgb(255, 120, 150);

    Theme {
        name: "Tokyo Night Moon".to_string(),
        is_light: false,

        bg_terminal: bg_dark,
        bg_base: bg,
        bg_surface: Color::Rgb(38, 42, 64),
        bg_elevated: bg_highlight,
        bg_highlight: Color::Rgb(55, 63, 95),
        markdown_code_bg: bg_dark,
        markdown_inline_code_bg: Color::Rgb(38, 42, 64),

        text_bright: fg,
        text_primary: fg_dark,
        text_secondary: dark5,
        text_muted: comment,
        text_faint: fg_gutter,

        accent_primary: blue,
        accent_secondary: magenta,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: magenta,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: magenta,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: dark3,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(110, 148, 228),
        spinner_trail_2: Color::Rgb(90, 126, 200),
        spinner_trail_3: Color::Rgb(72, 105, 172),
        spinner_trail_4: Color::Rgb(56, 84, 145),
        spinner_trail_5: Color::Rgb(42, 65, 118),
        spinner_inactive: comment,

        border_default: dark3,
        border_focused: blue,
        border_dimmed: Color::Rgb(40, 46, 70),

        shine_edge: comment,
        shine_mid: dark5,
        shine_center: fg_dark,
        shine_peak: fg,

        tool_block_bg: bg_dark,
        tool_comment: comment,
        tool_command: fg_dark,
        tool_output: dark5,
        diff_add: green,
        diff_remove: red,
    }
}

/// Tokyo Night Storm theme.
pub fn tokyo_night_storm() -> Theme {
    let bg = Color::Rgb(36, 40, 59);
    let bg_dark = Color::Rgb(29, 32, 46);
    let bg_highlight = Color::Rgb(47, 53, 79);
    let fg = Color::Rgb(192, 202, 245);
    let fg_dark = Color::Rgb(169, 177, 214);
    let fg_gutter = Color::Rgb(59, 66, 97);
    let dark3 = Color::Rgb(68, 75, 106);
    let comment = Color::Rgb(86, 95, 137);
    let dark5 = Color::Rgb(115, 125, 174);
    let blue = Color::Rgb(122, 162, 247);
    let blue0 = Color::Rgb(61, 89, 161);
    let cyan = Color::Rgb(125, 207, 255);
    let magenta = Color::Rgb(187, 154, 247);
    let green = Color::Rgb(158, 206, 106);
    let yellow = Color::Rgb(224, 175, 104);
    let red = Color::Rgb(247, 118, 142);

    Theme {
        name: "Tokyo Night Storm".to_string(),
        is_light: false,

        bg_terminal: bg_dark,
        bg_base: bg,
        bg_surface: Color::Rgb(42, 46, 68),
        bg_elevated: bg_highlight,
        bg_highlight: Color::Rgb(58, 65, 96),
        markdown_code_bg: bg_dark,
        markdown_inline_code_bg: Color::Rgb(42, 46, 68),

        text_bright: fg,
        text_primary: fg_dark,
        text_secondary: dark5,
        text_muted: comment,
        text_faint: fg_gutter,

        accent_primary: blue,
        accent_secondary: magenta,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: magenta,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: magenta,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: dark3,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(105, 143, 222),
        spinner_trail_2: Color::Rgb(86, 122, 196),
        spinner_trail_3: Color::Rgb(70, 102, 170),
        spinner_trail_4: Color::Rgb(55, 83, 144),
        spinner_trail_5: Color::Rgb(42, 65, 118),
        spinner_inactive: comment,

        border_default: dark3,
        border_focused: blue0,
        border_dimmed: Color::Rgb(42, 48, 72),

        shine_edge: comment,
        shine_mid: dark5,
        shine_center: fg_dark,
        shine_peak: fg,

        tool_block_bg: bg_dark,
        tool_comment: comment,
        tool_command: fg_dark,
        tool_output: dark5,
        diff_add: green,
        diff_remove: red,
    }
}

/// Tokyo Night Day theme (light).
pub fn tokyo_night_day() -> Theme {
    let bg = Color::Rgb(232, 235, 245);
    let bg_dark = Color::Rgb(215, 219, 235);
    let bg_highlight = Color::Rgb(195, 200, 222);
    let fg = Color::Rgb(56, 62, 98);
    let fg_dark = Color::Rgb(73, 78, 115);
    let comment = Color::Rgb(149, 157, 199);
    let blue = Color::Rgb(52, 99, 225);
    let cyan = Color::Rgb(7, 149, 190);
    let magenta = Color::Rgb(122, 71, 221);
    let green = Color::Rgb(72, 142, 60);
    let yellow = Color::Rgb(143, 96, 30);
    let red = Color::Rgb(207, 56, 100);

    Theme {
        name: "Tokyo Night Day".to_string(),
        is_light: true,

        bg_terminal: bg,
        bg_base: bg,
        bg_surface: bg_dark,
        bg_elevated: bg_highlight,
        bg_highlight: Color::Rgb(178, 184, 210),
        markdown_code_bg: bg_dark,
        markdown_inline_code_bg: bg_highlight,

        text_bright: fg,
        text_primary: fg_dark,
        text_secondary: Color::Rgb(100, 108, 155),
        text_muted: comment,
        text_faint: Color::Rgb(185, 190, 215),

        accent_primary: blue,
        accent_secondary: magenta,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: magenta,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: magenta,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: bg_highlight,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(80, 125, 235),
        spinner_trail_2: Color::Rgb(110, 150, 238),
        spinner_trail_3: Color::Rgb(140, 172, 240),
        spinner_trail_4: Color::Rgb(168, 192, 242),
        spinner_trail_5: Color::Rgb(196, 210, 244),
        spinner_inactive: comment,

        border_default: bg_highlight,
        border_focused: blue,
        border_dimmed: bg_dark,

        shine_edge: Color::Rgb(185, 190, 215),
        shine_mid: comment,
        shine_center: fg_dark,
        shine_peak: fg,

        tool_block_bg: bg_dark,
        tool_comment: comment,
        tool_command: fg,
        tool_output: fg_dark,
        diff_add: green,
        diff_remove: red,
    }
}

/// Rosé Pine theme.
pub fn rose_pine() -> Theme {
    let base = Color::Rgb(25, 23, 36);
    let surface = Color::Rgb(31, 29, 46);
    let overlay = Color::Rgb(38, 35, 58);
    let muted = Color::Rgb(110, 106, 134);
    let subtle = Color::Rgb(144, 140, 170);
    let text = Color::Rgb(224, 222, 244);
    let love = Color::Rgb(235, 111, 146);
    let gold = Color::Rgb(246, 193, 119);
    let pine = Color::Rgb(49, 116, 143);
    let foam = Color::Rgb(156, 207, 216);
    let iris = Color::Rgb(196, 167, 231);
    let hl_low = Color::Rgb(33, 32, 46);
    let hl_med = Color::Rgb(64, 61, 82);
    let _hl_high = Color::Rgb(82, 79, 103);

    Theme {
        name: "Rosé Pine".to_string(),
        is_light: false,

        bg_terminal: hl_low,
        bg_base: base,
        bg_surface: surface,
        bg_elevated: overlay,
        bg_highlight: hl_med,
        markdown_code_bg: hl_low,
        markdown_inline_code_bg: surface,

        text_bright: text,
        text_primary: Color::Rgb(210, 207, 230),
        text_secondary: subtle,
        text_muted: muted,
        text_faint: Color::Rgb(82, 79, 103),

        accent_primary: iris,
        accent_secondary: foam,
        accent_success: Color::Rgb(120, 190, 140),
        accent_warning: gold,
        accent_error: love,

        agent_claude: foam,
        agent_codex: iris,
        agent_opencode: pine,

        pr_open_bg: Color::Rgb(120, 190, 140),
        pr_merged_bg: iris,
        pr_closed_bg: love,
        pr_draft_bg: muted,
        pr_unknown_bg: overlay,

        spinner_active: iris,
        spinner_trail_1: Color::Rgb(172, 147, 208),
        spinner_trail_2: Color::Rgb(148, 128, 185),
        spinner_trail_3: Color::Rgb(124, 108, 162),
        spinner_trail_4: Color::Rgb(100, 88, 138),
        spinner_trail_5: Color::Rgb(78, 70, 114),
        spinner_inactive: muted,

        border_default: surface,
        border_focused: pine,
        border_dimmed: hl_low,

        shine_edge: muted,
        shine_mid: subtle,
        shine_center: Color::Rgb(210, 207, 230),
        shine_peak: text,

        tool_block_bg: hl_low,
        tool_comment: muted,
        tool_command: text,
        tool_output: subtle,
        diff_add: Color::Rgb(120, 190, 140),
        diff_remove: love,
    }
}

/// Rosé Pine Moon theme.
pub fn rose_pine_moon() -> Theme {
    let base = Color::Rgb(35, 33, 54);
    let surface = Color::Rgb(42, 39, 63);
    let overlay = Color::Rgb(57, 53, 82);
    let muted = Color::Rgb(110, 106, 134);
    let subtle = Color::Rgb(144, 140, 170);
    let text = Color::Rgb(224, 222, 244);
    let love = Color::Rgb(235, 111, 146);
    let gold = Color::Rgb(246, 193, 119);
    let pine = Color::Rgb(62, 143, 176);
    let foam = Color::Rgb(156, 207, 216);
    let iris = Color::Rgb(196, 167, 231);
    let hl_low = Color::Rgb(42, 40, 62);
    let hl_med = Color::Rgb(68, 65, 90);
    let _hl_high = Color::Rgb(86, 82, 110);

    Theme {
        name: "Rosé Pine Moon".to_string(),
        is_light: false,

        bg_terminal: hl_low,
        bg_base: base,
        bg_surface: surface,
        bg_elevated: overlay,
        bg_highlight: hl_med,
        markdown_code_bg: hl_low,
        markdown_inline_code_bg: surface,

        text_bright: text,
        text_primary: Color::Rgb(210, 207, 230),
        text_secondary: subtle,
        text_muted: muted,
        text_faint: Color::Rgb(86, 82, 110),

        accent_primary: iris,
        accent_secondary: foam,
        accent_success: Color::Rgb(120, 190, 140),
        accent_warning: gold,
        accent_error: love,

        agent_claude: foam,
        agent_codex: iris,
        agent_opencode: pine,

        pr_open_bg: Color::Rgb(120, 190, 140),
        pr_merged_bg: iris,
        pr_closed_bg: love,
        pr_draft_bg: muted,
        pr_unknown_bg: overlay,

        spinner_active: iris,
        spinner_trail_1: Color::Rgb(172, 147, 208),
        spinner_trail_2: Color::Rgb(148, 128, 185),
        spinner_trail_3: Color::Rgb(124, 108, 162),
        spinner_trail_4: Color::Rgb(100, 88, 138),
        spinner_trail_5: Color::Rgb(78, 70, 114),
        spinner_inactive: muted,

        border_default: surface,
        border_focused: pine,
        border_dimmed: hl_low,

        shine_edge: muted,
        shine_mid: subtle,
        shine_center: Color::Rgb(210, 207, 230),
        shine_peak: text,

        tool_block_bg: hl_low,
        tool_comment: muted,
        tool_command: text,
        tool_output: subtle,
        diff_add: Color::Rgb(120, 190, 140),
        diff_remove: love,
    }
}

/// Rosé Pine Dawn theme (light).
pub fn rose_pine_dawn() -> Theme {
    let base = Color::Rgb(250, 244, 237);
    let surface = Color::Rgb(255, 250, 243);
    let overlay = Color::Rgb(242, 233, 222);
    let muted = Color::Rgb(152, 147, 165);
    let subtle = Color::Rgb(121, 117, 147);
    let text = Color::Rgb(87, 82, 121);
    let love = Color::Rgb(180, 99, 122);
    let gold = Color::Rgb(234, 157, 52);
    let pine = Color::Rgb(40, 105, 131);
    let foam = Color::Rgb(86, 148, 159);
    let iris = Color::Rgb(144, 122, 169);
    let hl_low = Color::Rgb(242, 238, 232);
    let hl_med = Color::Rgb(220, 215, 205);

    Theme {
        name: "Rosé Pine Dawn".to_string(),
        is_light: true,

        bg_terminal: base,
        bg_base: base,
        bg_surface: surface,
        bg_elevated: overlay,
        bg_highlight: hl_med,
        markdown_code_bg: overlay,
        markdown_inline_code_bg: hl_med,

        text_bright: Color::Rgb(60, 55, 90),
        text_primary: text,
        text_secondary: subtle,
        text_muted: muted,
        text_faint: Color::Rgb(180, 175, 195),

        accent_primary: iris,
        accent_secondary: foam,
        accent_success: Color::Rgb(40, 130, 80),
        accent_warning: gold,
        accent_error: love,

        agent_claude: foam,
        agent_codex: iris,
        agent_opencode: pine,

        pr_open_bg: Color::Rgb(40, 130, 80),
        pr_merged_bg: iris,
        pr_closed_bg: love,
        pr_draft_bg: muted,
        pr_unknown_bg: overlay,

        spinner_active: iris,
        spinner_trail_1: Color::Rgb(162, 140, 185),
        spinner_trail_2: Color::Rgb(178, 158, 198),
        spinner_trail_3: Color::Rgb(194, 176, 210),
        spinner_trail_4: Color::Rgb(208, 194, 222),
        spinner_trail_5: Color::Rgb(222, 212, 234),
        spinner_inactive: muted,

        border_default: hl_med,
        border_focused: pine,
        border_dimmed: hl_low,

        shine_edge: Color::Rgb(180, 175, 195),
        shine_mid: muted,
        shine_center: subtle,
        shine_peak: text,

        tool_block_bg: hl_low,
        tool_comment: muted,
        tool_command: text,
        tool_output: subtle,
        diff_add: Color::Rgb(40, 130, 80),
        diff_remove: love,
    }
}

/// Solarized Dark theme.
pub fn solarized_dark() -> Theme {
    let base03 = Color::Rgb(0, 43, 54);
    let base02 = Color::Rgb(7, 54, 66);
    let base01 = Color::Rgb(88, 110, 117);
    let base0 = Color::Rgb(131, 148, 150);
    let base1 = Color::Rgb(147, 161, 161);
    let base2 = Color::Rgb(238, 232, 213);
    let base3 = Color::Rgb(253, 246, 227);
    let yellow = Color::Rgb(181, 137, 0);
    let red = Color::Rgb(220, 50, 47);
    let violet = Color::Rgb(108, 113, 196);
    let blue = Color::Rgb(38, 139, 210);
    let cyan = Color::Rgb(42, 161, 152);
    let green = Color::Rgb(133, 153, 0);

    Theme {
        name: "Solarized Dark".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(0, 33, 44),
        bg_base: base03,
        bg_surface: base02,
        bg_elevated: Color::Rgb(14, 66, 80),
        bg_highlight: Color::Rgb(21, 78, 95),
        markdown_code_bg: Color::Rgb(0, 33, 44),
        markdown_inline_code_bg: base02,

        text_bright: base3,
        text_primary: base2,
        text_secondary: base1,
        text_muted: base0,
        text_faint: base01,

        accent_primary: blue,
        accent_secondary: violet,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: violet,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: violet,
        pr_closed_bg: red,
        pr_draft_bg: base01,
        pr_unknown_bg: base02,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(32, 118, 183),
        spinner_trail_2: Color::Rgb(27, 98, 155),
        spinner_trail_3: Color::Rgb(22, 80, 128),
        spinner_trail_4: Color::Rgb(18, 63, 102),
        spinner_trail_5: Color::Rgb(14, 48, 78),
        spinner_inactive: base01,

        border_default: base02,
        border_focused: blue,
        border_dimmed: Color::Rgb(5, 47, 58),

        shine_edge: base01,
        shine_mid: base0,
        shine_center: base1,
        shine_peak: base3,

        tool_block_bg: Color::Rgb(0, 33, 44),
        tool_comment: base01,
        tool_command: base2,
        tool_output: base1,
        diff_add: green,
        diff_remove: red,
    }
}

/// Solarized Light theme.
pub fn solarized_light() -> Theme {
    let base03 = Color::Rgb(0, 43, 54);
    let base02 = Color::Rgb(7, 54, 66);
    let base01 = Color::Rgb(88, 110, 117);
    let base00 = Color::Rgb(101, 123, 131);
    let base0 = Color::Rgb(131, 148, 150);
    let base1 = Color::Rgb(147, 161, 161);
    let base2 = Color::Rgb(238, 232, 213);
    let base3 = Color::Rgb(253, 246, 227);
    let yellow = Color::Rgb(181, 137, 0);
    let red = Color::Rgb(220, 50, 47);
    let violet = Color::Rgb(108, 113, 196);
    let blue = Color::Rgb(38, 139, 210);
    let cyan = Color::Rgb(42, 161, 152);
    let green = Color::Rgb(133, 153, 0);

    Theme {
        name: "Solarized Light".to_string(),
        is_light: true,

        bg_terminal: base3,
        bg_base: base3,
        bg_surface: base2,
        bg_elevated: Color::Rgb(228, 222, 200),
        bg_highlight: Color::Rgb(210, 205, 185),
        markdown_code_bg: base2,
        markdown_inline_code_bg: Color::Rgb(228, 222, 200),

        text_bright: base03,
        text_primary: base02,
        text_secondary: base01,
        text_muted: base00,
        text_faint: base0,

        accent_primary: blue,
        accent_secondary: violet,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: violet,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: violet,
        pr_closed_bg: red,
        pr_draft_bg: base1,
        pr_unknown_bg: base2,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(65, 152, 218),
        spinner_trail_2: Color::Rgb(95, 168, 224),
        spinner_trail_3: Color::Rgb(125, 184, 230),
        spinner_trail_4: Color::Rgb(155, 198, 236),
        spinner_trail_5: Color::Rgb(185, 214, 240),
        spinner_inactive: base1,

        border_default: Color::Rgb(210, 205, 185),
        border_focused: blue,
        border_dimmed: base2,

        shine_edge: base0,
        shine_mid: base01,
        shine_center: base02,
        shine_peak: base03,

        tool_block_bg: base2,
        tool_comment: base1,
        tool_command: base02,
        tool_output: base01,
        diff_add: green,
        diff_remove: red,
    }
}

/// Gruvbox Dark theme.
pub fn gruvbox_dark() -> Theme {
    let bg0_h = Color::Rgb(29, 32, 33);
    let bg0 = Color::Rgb(40, 40, 40);
    let bg1 = Color::Rgb(60, 56, 54);
    let bg2 = Color::Rgb(80, 73, 69);
    let bg3 = Color::Rgb(102, 92, 84);
    let bg4 = Color::Rgb(124, 111, 100);
    let fg0 = Color::Rgb(251, 241, 199);
    let fg1 = Color::Rgb(235, 219, 178);
    let fg2 = Color::Rgb(213, 196, 161);
    let fg3 = Color::Rgb(189, 174, 147);
    let fg4 = Color::Rgb(168, 153, 132);
    let red = Color::Rgb(251, 73, 52);
    let green = Color::Rgb(184, 187, 38);
    let yellow = Color::Rgb(250, 189, 47);
    let blue = Color::Rgb(131, 165, 152);
    let purple = Color::Rgb(211, 134, 155);
    let aqua = Color::Rgb(142, 192, 124);
    let orange = Color::Rgb(254, 128, 25);
    let gray = Color::Rgb(168, 153, 132);

    Theme {
        name: "Gruvbox Dark".to_string(),
        is_light: false,

        bg_terminal: bg0_h,
        bg_base: bg0,
        bg_surface: bg1,
        bg_elevated: bg2,
        bg_highlight: bg3,
        markdown_code_bg: bg0_h,
        markdown_inline_code_bg: bg1,

        text_bright: fg0,
        text_primary: fg1,
        text_secondary: fg2,
        text_muted: fg4,
        text_faint: bg4,

        accent_primary: yellow,
        accent_secondary: orange,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: aqua,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: gray,
        pr_unknown_bg: bg3,

        spinner_active: yellow,
        spinner_trail_1: Color::Rgb(220, 165, 40),
        spinner_trail_2: Color::Rgb(188, 140, 35),
        spinner_trail_3: Color::Rgb(155, 114, 28),
        spinner_trail_4: Color::Rgb(122, 90, 22),
        spinner_trail_5: Color::Rgb(90, 70, 20),
        spinner_inactive: bg4,

        border_default: bg2,
        border_focused: yellow,
        border_dimmed: bg1,

        shine_edge: bg4,
        shine_mid: fg4,
        shine_center: fg2,
        shine_peak: fg0,

        tool_block_bg: bg0_h,
        tool_comment: bg4,
        tool_command: fg1,
        tool_output: fg3,
        diff_add: green,
        diff_remove: red,
    }
}

/// Gruvbox Light theme.
pub fn gruvbox_light() -> Theme {
    let bg0_h = Color::Rgb(249, 245, 215);
    let bg0 = Color::Rgb(251, 241, 199);
    let bg1 = Color::Rgb(242, 229, 188);
    let bg2 = Color::Rgb(213, 196, 161);
    let bg3 = Color::Rgb(189, 174, 147);
    let bg4 = Color::Rgb(168, 153, 132);
    let fg0 = Color::Rgb(40, 40, 40);
    let fg1 = Color::Rgb(60, 56, 54);
    let fg2 = Color::Rgb(80, 73, 69);
    let fg3 = Color::Rgb(102, 92, 84);
    let fg4 = Color::Rgb(124, 111, 100);
    let red = Color::Rgb(204, 36, 29);
    let green = Color::Rgb(121, 116, 14);
    let yellow = Color::Rgb(181, 118, 20);
    let blue = Color::Rgb(7, 102, 120);
    let purple = Color::Rgb(143, 63, 113);
    let aqua = Color::Rgb(66, 123, 88);
    let orange = Color::Rgb(175, 58, 3);
    let gray = Color::Rgb(124, 111, 100);

    Theme {
        name: "Gruvbox Light".to_string(),
        is_light: true,

        bg_terminal: bg0_h,
        bg_base: bg0,
        bg_surface: bg1,
        bg_elevated: bg2,
        bg_highlight: bg3,
        markdown_code_bg: bg0_h,
        markdown_inline_code_bg: bg1,

        text_bright: fg0,
        text_primary: fg1,
        text_secondary: fg2,
        text_muted: fg4,
        text_faint: bg4,

        accent_primary: yellow,
        accent_secondary: orange,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: aqua,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: gray,
        pr_unknown_bg: bg3,

        spinner_active: yellow,
        spinner_trail_1: Color::Rgb(198, 135, 40),
        spinner_trail_2: Color::Rgb(212, 150, 65),
        spinner_trail_3: Color::Rgb(224, 165, 90),
        spinner_trail_4: Color::Rgb(234, 180, 115),
        spinner_trail_5: Color::Rgb(242, 195, 140),
        spinner_inactive: bg4,

        border_default: bg2,
        border_focused: yellow,
        border_dimmed: bg1,

        shine_edge: bg4,
        shine_mid: fg4,
        shine_center: fg2,
        shine_peak: fg0,

        tool_block_bg: bg0_h,
        tool_comment: bg4,
        tool_command: fg1,
        tool_output: fg3,
        diff_add: green,
        diff_remove: red,
    }
}

/// Monokai Classic theme.
pub fn monokai() -> Theme {
    let background = Color::Rgb(39, 40, 34);
    let bg_light = Color::Rgb(50, 51, 44);
    let selection = Color::Rgb(73, 72, 62);
    let foreground = Color::Rgb(248, 248, 242);
    let comment = Color::Rgb(117, 113, 94);
    let red = Color::Rgb(249, 38, 114);
    let orange = Color::Rgb(253, 151, 31);
    let green = Color::Rgb(166, 226, 46);
    let blue = Color::Rgb(102, 217, 232);
    let purple = Color::Rgb(174, 129, 255);

    Theme {
        name: "Monokai".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(30, 31, 26),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(90, 89, 78),
        markdown_code_bg: Color::Rgb(30, 31, 26),
        markdown_inline_code_bg: bg_light,

        text_bright: foreground,
        text_primary: Color::Rgb(232, 232, 225),
        text_secondary: Color::Rgb(190, 185, 170),
        text_muted: comment,
        text_faint: Color::Rgb(80, 78, 65),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: orange,
        accent_error: red,

        agent_claude: blue,
        agent_codex: purple,
        agent_opencode: green,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(85, 185, 202),
        spinner_trail_2: Color::Rgb(70, 158, 174),
        spinner_trail_3: Color::Rgb(56, 132, 146),
        spinner_trail_4: Color::Rgb(43, 106, 118),
        spinner_trail_5: Color::Rgb(32, 82, 92),
        spinner_inactive: comment,

        border_default: bg_light,
        border_focused: blue,
        border_dimmed: Color::Rgb(45, 46, 40),

        shine_edge: comment,
        shine_mid: Color::Rgb(160, 155, 140),
        shine_center: Color::Rgb(210, 207, 195),
        shine_peak: foreground,

        tool_block_bg: Color::Rgb(30, 31, 26),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(190, 185, 170),
        diff_add: green,
        diff_remove: red,
    }
}

/// Monokai Pro theme.
pub fn monokai_pro() -> Theme {
    let background = Color::Rgb(40, 42, 54);
    let bg_light = Color::Rgb(50, 52, 65);
    let selection = Color::Rgb(63, 66, 87);
    let foreground = Color::Rgb(252, 252, 250);
    let comment = Color::Rgb(122, 121, 127);
    let red = Color::Rgb(255, 97, 136);
    let orange = Color::Rgb(255, 157, 69);
    let _yellow = Color::Rgb(255, 216, 102);
    let green = Color::Rgb(169, 220, 118);
    let blue = Color::Rgb(120, 220, 232);
    let purple = Color::Rgb(171, 157, 242);

    Theme {
        name: "Monokai Pro".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(31, 33, 44),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(76, 80, 104),
        markdown_code_bg: Color::Rgb(31, 33, 44),
        markdown_inline_code_bg: bg_light,

        text_bright: foreground,
        text_primary: Color::Rgb(230, 230, 228),
        text_secondary: Color::Rgb(185, 183, 195),
        text_muted: comment,
        text_faint: Color::Rgb(80, 79, 95),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: orange,
        accent_error: red,

        agent_claude: blue,
        agent_codex: purple,
        agent_opencode: green,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(100, 195, 208),
        spinner_trail_2: Color::Rgb(82, 168, 180),
        spinner_trail_3: Color::Rgb(66, 142, 152),
        spinner_trail_4: Color::Rgb(52, 116, 125),
        spinner_trail_5: Color::Rgb(40, 92, 100),
        spinner_inactive: comment,

        border_default: bg_light,
        border_focused: blue,
        border_dimmed: Color::Rgb(38, 40, 52),

        shine_edge: comment,
        shine_mid: Color::Rgb(165, 162, 175),
        shine_center: Color::Rgb(210, 208, 218),
        shine_peak: foreground,

        tool_block_bg: Color::Rgb(31, 33, 44),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(185, 183, 195),
        diff_add: green,
        diff_remove: red,
    }
}

/// Monokai Vivid theme.
pub fn monokai_vivid() -> Theme {
    let background = Color::Rgb(37, 37, 37);
    let bg_light = Color::Rgb(52, 52, 52);
    let selection = Color::Rgb(65, 65, 65);
    let foreground = Color::Rgb(255, 255, 255);
    let comment = Color::Rgb(96, 96, 96);
    let red = Color::Rgb(255, 0, 86);
    let _orange = Color::Rgb(255, 157, 0);
    let yellow = Color::Rgb(255, 231, 0);
    let green = Color::Rgb(141, 255, 0);
    let blue = Color::Rgb(0, 249, 255);
    let purple = Color::Rgb(184, 0, 255);

    Theme {
        name: "Monokai Vivid".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(28, 28, 28),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(80, 80, 80),
        markdown_code_bg: Color::Rgb(28, 28, 28),
        markdown_inline_code_bg: bg_light,

        text_bright: foreground,
        text_primary: Color::Rgb(230, 230, 230),
        text_secondary: Color::Rgb(180, 180, 180),
        text_muted: comment,
        text_faint: Color::Rgb(65, 65, 65),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: blue,
        agent_codex: purple,
        agent_opencode: green,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(0, 210, 215),
        spinner_trail_2: Color::Rgb(0, 172, 178),
        spinner_trail_3: Color::Rgb(0, 136, 142),
        spinner_trail_4: Color::Rgb(0, 102, 108),
        spinner_trail_5: Color::Rgb(0, 72, 76),
        spinner_inactive: comment,

        border_default: bg_light,
        border_focused: blue,
        border_dimmed: Color::Rgb(42, 42, 42),

        shine_edge: comment,
        shine_mid: Color::Rgb(155, 155, 155),
        shine_center: Color::Rgb(210, 210, 210),
        shine_peak: foreground,

        tool_block_bg: Color::Rgb(28, 28, 28),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(180, 180, 180),
        diff_add: green,
        diff_remove: red,
    }
}

/// Tomorrow Night theme.
pub fn tomorrow_night() -> Theme {
    let background = Color::Rgb(29, 31, 33);
    let current_line = Color::Rgb(40, 42, 46);
    let selection = Color::Rgb(55, 59, 65);
    let foreground = Color::Rgb(197, 200, 198);
    let comment = Color::Rgb(150, 152, 150);
    let red = Color::Rgb(204, 102, 102);
    let yellow = Color::Rgb(240, 198, 116);
    let green = Color::Rgb(181, 189, 104);
    let aqua = Color::Rgb(138, 190, 183);
    let blue = Color::Rgb(129, 162, 190);
    let purple = Color::Rgb(178, 148, 187);

    Theme {
        name: "Tomorrow Night".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(22, 23, 25),
        bg_base: background,
        bg_surface: current_line,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(68, 73, 80),
        markdown_code_bg: Color::Rgb(22, 23, 25),
        markdown_inline_code_bg: current_line,

        text_bright: foreground,
        text_primary: Color::Rgb(185, 188, 186),
        text_secondary: comment,
        text_muted: Color::Rgb(110, 112, 110),
        text_faint: Color::Rgb(75, 78, 75),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: aqua,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(108, 140, 165),
        spinner_trail_2: Color::Rgb(88, 118, 142),
        spinner_trail_3: Color::Rgb(70, 97, 120),
        spinner_trail_4: Color::Rgb(55, 78, 98),
        spinner_trail_5: Color::Rgb(42, 61, 78),
        spinner_inactive: comment,

        border_default: current_line,
        border_focused: blue,
        border_dimmed: Color::Rgb(35, 37, 40),

        shine_edge: comment,
        shine_mid: Color::Rgb(165, 168, 166),
        shine_center: foreground,
        shine_peak: Color::Rgb(230, 232, 230),

        tool_block_bg: Color::Rgb(22, 23, 25),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(165, 168, 166),
        diff_add: green,
        diff_remove: red,
    }
}

/// Tomorrow Night Eighties theme.
pub fn tomorrow_night_eighties() -> Theme {
    let background = Color::Rgb(45, 45, 45);
    let current_line = Color::Rgb(57, 57, 57);
    let selection = Color::Rgb(81, 81, 81);
    let foreground = Color::Rgb(204, 204, 204);
    let comment = Color::Rgb(153, 153, 153);
    let red = Color::Rgb(242, 119, 122);
    let yellow = Color::Rgb(255, 204, 102);
    let green = Color::Rgb(153, 204, 153);
    let aqua = Color::Rgb(102, 204, 204);
    let blue = Color::Rgb(102, 153, 204);
    let purple = Color::Rgb(204, 153, 204);

    Theme {
        name: "Tomorrow Night Eighties".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(35, 35, 35),
        bg_base: background,
        bg_surface: current_line,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(96, 96, 96),
        markdown_code_bg: Color::Rgb(35, 35, 35),
        markdown_inline_code_bg: current_line,

        text_bright: foreground,
        text_primary: Color::Rgb(190, 190, 190),
        text_secondary: comment,
        text_muted: Color::Rgb(118, 118, 118),
        text_faint: Color::Rgb(80, 80, 80),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: aqua,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(85, 130, 178),
        spinner_trail_2: Color::Rgb(70, 108, 152),
        spinner_trail_3: Color::Rgb(57, 88, 126),
        spinner_trail_4: Color::Rgb(45, 70, 102),
        spinner_trail_5: Color::Rgb(35, 54, 80),
        spinner_inactive: comment,

        border_default: current_line,
        border_focused: blue,
        border_dimmed: Color::Rgb(50, 50, 50),

        shine_edge: comment,
        shine_mid: Color::Rgb(175, 175, 175),
        shine_center: foreground,
        shine_peak: Color::Rgb(230, 230, 230),

        tool_block_bg: Color::Rgb(35, 35, 35),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(175, 175, 175),
        diff_add: green,
        diff_remove: red,
    }
}

/// Tomorrow Night Blue theme.
pub fn tomorrow_night_blue() -> Theme {
    let background = Color::Rgb(0, 39, 68);
    let current_line = Color::Rgb(0, 51, 88);
    let selection = Color::Rgb(0, 71, 116);
    let foreground = Color::Rgb(170, 196, 220);
    let comment = Color::Rgb(124, 163, 197);
    let red = Color::Rgb(255, 145, 143);
    let yellow = Color::Rgb(255, 220, 135);
    let green = Color::Rgb(189, 230, 148);
    let aqua = Color::Rgb(150, 230, 225);
    let blue = Color::Rgb(150, 196, 236);
    let purple = Color::Rgb(210, 175, 220);

    Theme {
        name: "Tomorrow Night Blue".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(0, 28, 50),
        bg_base: background,
        bg_surface: current_line,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(0, 88, 142),
        markdown_code_bg: Color::Rgb(0, 28, 50),
        markdown_inline_code_bg: current_line,

        text_bright: foreground,
        text_primary: Color::Rgb(155, 183, 210),
        text_secondary: comment,
        text_muted: Color::Rgb(90, 130, 165),
        text_faint: Color::Rgb(50, 88, 118),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: aqua,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(126, 170, 212),
        spinner_trail_2: Color::Rgb(103, 145, 188),
        spinner_trail_3: Color::Rgb(82, 122, 164),
        spinner_trail_4: Color::Rgb(63, 100, 140),
        spinner_trail_5: Color::Rgb(46, 80, 116),
        spinner_inactive: comment,

        border_default: current_line,
        border_focused: blue,
        border_dimmed: Color::Rgb(0, 40, 65),

        shine_edge: comment,
        shine_mid: Color::Rgb(148, 178, 208),
        shine_center: foreground,
        shine_peak: Color::Rgb(210, 228, 244),

        tool_block_bg: Color::Rgb(0, 28, 50),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(148, 178, 208),
        diff_add: green,
        diff_remove: red,
    }
}

/// Tomorrow Night Bright theme.
pub fn tomorrow_night_bright() -> Theme {
    let background = Color::Rgb(0, 0, 0);
    let current_line = Color::Rgb(20, 20, 20);
    let selection = Color::Rgb(35, 35, 35);
    let foreground = Color::Rgb(234, 234, 234);
    let comment = Color::Rgb(150, 150, 150);
    let red = Color::Rgb(216, 98, 98);
    let yellow = Color::Rgb(245, 206, 120);
    let green = Color::Rgb(185, 195, 100);
    let aqua = Color::Rgb(138, 195, 188);
    let blue = Color::Rgb(126, 162, 192);
    let purple = Color::Rgb(183, 152, 192);

    Theme {
        name: "Tomorrow Night Bright".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(0, 0, 0),
        bg_base: background,
        bg_surface: current_line,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(50, 50, 50),
        markdown_code_bg: Color::Rgb(0, 0, 0),
        markdown_inline_code_bg: current_line,

        text_bright: foreground,
        text_primary: Color::Rgb(210, 210, 210),
        text_secondary: comment,
        text_muted: Color::Rgb(110, 110, 110),
        text_faint: Color::Rgb(70, 70, 70),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: aqua,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(105, 138, 165),
        spinner_trail_2: Color::Rgb(86, 114, 138),
        spinner_trail_3: Color::Rgb(68, 92, 112),
        spinner_trail_4: Color::Rgb(52, 72, 88),
        spinner_trail_5: Color::Rgb(38, 55, 68),
        spinner_inactive: comment,

        border_default: current_line,
        border_focused: blue,
        border_dimmed: Color::Rgb(28, 28, 28),

        shine_edge: comment,
        shine_mid: Color::Rgb(178, 178, 178),
        shine_center: foreground,
        shine_peak: Color::Rgb(255, 255, 255),

        tool_block_bg: Color::Rgb(0, 0, 0),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(178, 178, 178),
        diff_add: green,
        diff_remove: red,
    }
}

/// Atom One Dark theme.
pub fn atom_one_dark() -> Theme {
    let background = Color::Rgb(40, 44, 52);
    let bg_light = Color::Rgb(49, 53, 61);
    let selection = Color::Rgb(62, 68, 81);
    let foreground = Color::Rgb(171, 178, 191);
    let comment = Color::Rgb(92, 99, 112);
    let red = Color::Rgb(224, 108, 117);
    let yellow = Color::Rgb(229, 192, 123);
    let green = Color::Rgb(152, 195, 121);
    let cyan = Color::Rgb(86, 182, 194);
    let blue = Color::Rgb(97, 175, 239);
    let purple = Color::Rgb(198, 120, 221);

    Theme {
        name: "Atom One Dark".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(30, 34, 40),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(75, 82, 97),
        markdown_code_bg: Color::Rgb(30, 34, 40),
        markdown_inline_code_bg: bg_light,

        text_bright: Color::Rgb(220, 225, 236),
        text_primary: foreground,
        text_secondary: Color::Rgb(140, 147, 160),
        text_muted: comment,
        text_faint: Color::Rgb(65, 72, 85),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(80, 152, 210),
        spinner_trail_2: Color::Rgb(65, 130, 182),
        spinner_trail_3: Color::Rgb(52, 110, 155),
        spinner_trail_4: Color::Rgb(40, 90, 128),
        spinner_trail_5: Color::Rgb(30, 72, 102),
        spinner_inactive: comment,

        border_default: bg_light,
        border_focused: blue,
        border_dimmed: Color::Rgb(38, 42, 50),

        shine_edge: comment,
        shine_mid: Color::Rgb(140, 147, 160),
        shine_center: foreground,
        shine_peak: Color::Rgb(220, 225, 236),

        tool_block_bg: Color::Rgb(30, 34, 40),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(140, 147, 160),
        diff_add: green,
        diff_remove: red,
    }
}

/// One Half Dark theme.
pub fn one_half_dark() -> Theme {
    let background = Color::Rgb(40, 44, 52);
    let bg_light = Color::Rgb(50, 55, 64);
    let selection = Color::Rgb(65, 70, 82);
    let foreground = Color::Rgb(220, 223, 228);
    let comment = Color::Rgb(90, 100, 120);
    let red = Color::Rgb(228, 86, 93);
    let yellow = Color::Rgb(229, 192, 123);
    let green = Color::Rgb(152, 195, 121);
    let cyan = Color::Rgb(86, 182, 194);
    let blue = Color::Rgb(97, 175, 239);
    let purple = Color::Rgb(198, 120, 221);

    Theme {
        name: "One Half Dark".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(30, 34, 40),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(78, 85, 100),
        markdown_code_bg: Color::Rgb(30, 34, 40),
        markdown_inline_code_bg: bg_light,

        text_bright: foreground,
        text_primary: Color::Rgb(195, 200, 210),
        text_secondary: Color::Rgb(148, 155, 170),
        text_muted: comment,
        text_faint: Color::Rgb(65, 72, 85),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(80, 152, 210),
        spinner_trail_2: Color::Rgb(65, 130, 182),
        spinner_trail_3: Color::Rgb(52, 110, 155),
        spinner_trail_4: Color::Rgb(40, 90, 128),
        spinner_trail_5: Color::Rgb(30, 72, 102),
        spinner_inactive: comment,

        border_default: bg_light,
        border_focused: blue,
        border_dimmed: Color::Rgb(38, 42, 50),

        shine_edge: comment,
        shine_mid: Color::Rgb(148, 155, 170),
        shine_center: Color::Rgb(195, 200, 210),
        shine_peak: foreground,

        tool_block_bg: Color::Rgb(30, 34, 40),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(148, 155, 170),
        diff_add: green,
        diff_remove: red,
    }
}

/// GitHub Dark theme.
pub fn github_dark() -> Theme {
    let bg = Color::Rgb(13, 17, 23);
    let bg_secondary = Color::Rgb(22, 27, 34);
    let bg_tertiary = Color::Rgb(33, 38, 45);
    let border = Color::Rgb(48, 54, 61);
    let fg = Color::Rgb(230, 237, 243);
    let fg_muted = Color::Rgb(139, 148, 158);
    let fg_subtle = Color::Rgb(110, 118, 129);
    let accent = Color::Rgb(88, 166, 255);
    let success = Color::Rgb(63, 185, 80);
    let warning = Color::Rgb(210, 153, 34);
    let danger = Color::Rgb(248, 81, 73);
    let done = Color::Rgb(163, 113, 247);

    Theme {
        name: "GitHub Dark".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(1, 4, 9),
        bg_base: bg,
        bg_surface: bg_secondary,
        bg_elevated: bg_tertiary,
        bg_highlight: border,
        markdown_code_bg: Color::Rgb(1, 4, 9),
        markdown_inline_code_bg: bg_secondary,

        text_bright: fg,
        text_primary: Color::Rgb(200, 210, 220),
        text_secondary: fg_muted,
        text_muted: fg_subtle,
        text_faint: Color::Rgb(80, 90, 100),

        accent_primary: accent,
        accent_secondary: done,
        accent_success: success,
        accent_warning: warning,
        accent_error: danger,

        agent_claude: Color::Rgb(88, 196, 220),
        agent_codex: done,
        agent_opencode: accent,

        pr_open_bg: success,
        pr_merged_bg: done,
        pr_closed_bg: danger,
        pr_draft_bg: fg_subtle,
        pr_unknown_bg: bg_tertiary,

        spinner_active: accent,
        spinner_trail_1: Color::Rgb(72, 142, 228),
        spinner_trail_2: Color::Rgb(58, 120, 200),
        spinner_trail_3: Color::Rgb(46, 100, 172),
        spinner_trail_4: Color::Rgb(36, 80, 144),
        spinner_trail_5: Color::Rgb(28, 62, 116),
        spinner_inactive: fg_subtle,

        border_default: border,
        border_focused: accent,
        border_dimmed: bg_tertiary,

        shine_edge: fg_subtle,
        shine_mid: fg_muted,
        shine_center: Color::Rgb(200, 210, 220),
        shine_peak: fg,

        tool_block_bg: Color::Rgb(1, 4, 9),
        tool_comment: fg_subtle,
        tool_command: fg,
        tool_output: fg_muted,
        diff_add: success,
        diff_remove: danger,
    }
}

/// GitHub Light theme.
pub fn github_light() -> Theme {
    let bg = Color::Rgb(255, 255, 255);
    let bg_secondary = Color::Rgb(246, 248, 250);
    let bg_tertiary = Color::Rgb(235, 238, 242);
    let border = Color::Rgb(208, 215, 222);
    let fg = Color::Rgb(31, 35, 40);
    let fg_muted = Color::Rgb(99, 108, 118);
    let fg_subtle = Color::Rgb(110, 119, 129);
    let accent = Color::Rgb(9, 105, 218);
    let success = Color::Rgb(26, 127, 55);
    let warning = Color::Rgb(154, 103, 0);
    let danger = Color::Rgb(209, 36, 47);
    let done = Color::Rgb(130, 80, 223);

    Theme {
        name: "GitHub Light".to_string(),
        is_light: true,

        bg_terminal: bg,
        bg_base: bg,
        bg_surface: bg_secondary,
        bg_elevated: bg_tertiary,
        bg_highlight: border,
        markdown_code_bg: bg_secondary,
        markdown_inline_code_bg: bg_tertiary,

        text_bright: fg,
        text_primary: Color::Rgb(55, 60, 65),
        text_secondary: fg_muted,
        text_muted: fg_subtle,
        text_faint: Color::Rgb(170, 178, 186),

        accent_primary: accent,
        accent_secondary: done,
        accent_success: success,
        accent_warning: warning,
        accent_error: danger,

        agent_claude: Color::Rgb(15, 148, 175),
        agent_codex: done,
        agent_opencode: accent,

        pr_open_bg: success,
        pr_merged_bg: done,
        pr_closed_bg: danger,
        pr_draft_bg: fg_subtle,
        pr_unknown_bg: bg_tertiary,

        spinner_active: accent,
        spinner_trail_1: Color::Rgb(50, 128, 230),
        spinner_trail_2: Color::Rgb(92, 152, 232),
        spinner_trail_3: Color::Rgb(134, 176, 234),
        spinner_trail_4: Color::Rgb(172, 198, 238),
        spinner_trail_5: Color::Rgb(208, 220, 242),
        spinner_inactive: fg_subtle,

        border_default: border,
        border_focused: accent,
        border_dimmed: bg_tertiary,

        shine_edge: Color::Rgb(170, 178, 186),
        shine_mid: fg_muted,
        shine_center: Color::Rgb(55, 60, 65),
        shine_peak: fg,

        tool_block_bg: bg_secondary,
        tool_comment: fg_subtle,
        tool_command: fg,
        tool_output: fg_muted,
        diff_add: success,
        diff_remove: danger,
    }
}

/// JetBrains Darcula theme.
pub fn jetbrains_darcula() -> Theme {
    let background = Color::Rgb(43, 43, 43);
    let bg_lighter = Color::Rgb(55, 55, 55);
    let selection = Color::Rgb(69, 73, 74);
    let foreground = Color::Rgb(169, 183, 198);
    let comment = Color::Rgb(128, 128, 128);
    let string = Color::Rgb(106, 135, 89);
    let number = Color::Rgb(104, 151, 187);
    let func = Color::Rgb(255, 198, 109);
    let constant = Color::Rgb(152, 118, 170);
    let error = Color::Rgb(255, 100, 100);

    Theme {
        name: "JetBrains Darcula".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(33, 33, 33),
        bg_base: background,
        bg_surface: bg_lighter,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(82, 87, 88),
        markdown_code_bg: Color::Rgb(33, 33, 33),
        markdown_inline_code_bg: bg_lighter,

        text_bright: Color::Rgb(215, 225, 238),
        text_primary: foreground,
        text_secondary: Color::Rgb(135, 145, 160),
        text_muted: comment,
        text_faint: Color::Rgb(85, 90, 95),

        accent_primary: number,
        accent_secondary: constant,
        accent_success: string,
        accent_warning: func,
        accent_error: error,

        agent_claude: Color::Rgb(80, 180, 180),
        agent_codex: constant,
        agent_opencode: number,

        pr_open_bg: string,
        pr_merged_bg: constant,
        pr_closed_bg: error,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: number,
        spinner_trail_1: Color::Rgb(86, 130, 165),
        spinner_trail_2: Color::Rgb(70, 110, 142),
        spinner_trail_3: Color::Rgb(56, 90, 120),
        spinner_trail_4: Color::Rgb(44, 72, 98),
        spinner_trail_5: Color::Rgb(34, 56, 78),
        spinner_inactive: comment,

        border_default: bg_lighter,
        border_focused: number,
        border_dimmed: Color::Rgb(45, 45, 45),

        shine_edge: comment,
        shine_mid: Color::Rgb(150, 160, 175),
        shine_center: foreground,
        shine_peak: Color::Rgb(215, 225, 238),

        tool_block_bg: Color::Rgb(33, 33, 33),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(135, 145, 160),
        diff_add: string,
        diff_remove: error,
    }
}

/// VS Code Dark+ theme.
pub fn vscode_dark() -> Theme {
    let bg = Color::Rgb(30, 30, 30);
    let bg_secondary = Color::Rgb(37, 37, 38);
    let bg_active = Color::Rgb(44, 44, 44);
    let bg_highlight = Color::Rgb(55, 55, 60);
    let fg = Color::Rgb(212, 212, 212);
    let comment = Color::Rgb(106, 153, 85);
    let keyword = Color::Rgb(86, 156, 214);
    let variable = Color::Rgb(156, 220, 254);
    let type_color = Color::Rgb(78, 201, 176);
    let func = Color::Rgb(220, 220, 170);
    let error = Color::Rgb(244, 71, 71);

    Theme {
        name: "VS Code Dark+".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(24, 24, 24),
        bg_base: bg,
        bg_surface: bg_secondary,
        bg_elevated: bg_active,
        bg_highlight: bg_highlight,
        markdown_code_bg: Color::Rgb(24, 24, 24),
        markdown_inline_code_bg: bg_secondary,

        text_bright: Color::Rgb(255, 255, 255),
        text_primary: fg,
        text_secondary: Color::Rgb(180, 180, 180),
        text_muted: Color::Rgb(130, 130, 130),
        text_faint: Color::Rgb(90, 90, 90),

        accent_primary: keyword,
        accent_secondary: variable,
        accent_success: type_color,
        accent_warning: func,
        accent_error: error,

        agent_claude: type_color,
        agent_codex: variable,
        agent_opencode: keyword,

        pr_open_bg: Color::Rgb(70, 180, 80),
        pr_merged_bg: Color::Rgb(130, 80, 200),
        pr_closed_bg: error,
        pr_draft_bg: Color::Rgb(130, 130, 130),
        pr_unknown_bg: bg_active,

        spinner_active: keyword,
        spinner_trail_1: Color::Rgb(70, 134, 190),
        spinner_trail_2: Color::Rgb(56, 113, 165),
        spinner_trail_3: Color::Rgb(44, 93, 140),
        spinner_trail_4: Color::Rgb(34, 75, 115),
        spinner_trail_5: Color::Rgb(26, 58, 92),
        spinner_inactive: Color::Rgb(130, 130, 130),

        border_default: bg_secondary,
        border_focused: keyword,
        border_dimmed: Color::Rgb(35, 35, 35),

        shine_edge: Color::Rgb(100, 100, 100),
        shine_mid: Color::Rgb(155, 155, 155),
        shine_center: fg,
        shine_peak: Color::Rgb(255, 255, 255),

        tool_block_bg: Color::Rgb(24, 24, 24),
        tool_comment: comment,
        tool_command: fg,
        tool_output: Color::Rgb(180, 180, 180),
        diff_add: Color::Rgb(70, 180, 80),
        diff_remove: error,
    }
}

/// VS Code Dark Modern theme.
pub fn vscode_dark_modern() -> Theme {
    let bg = Color::Rgb(30, 30, 30);
    let bg_secondary = Color::Rgb(40, 44, 52);
    let bg_active = Color::Rgb(50, 54, 64);
    let bg_highlight = Color::Rgb(62, 68, 80);
    let fg = Color::Rgb(204, 204, 204);
    let comment = Color::Rgb(113, 165, 90);
    let keyword = Color::Rgb(86, 156, 214);
    let variable = Color::Rgb(156, 220, 254);
    let type_color = Color::Rgb(78, 201, 176);
    let func = Color::Rgb(220, 220, 170);
    let error = Color::Rgb(244, 71, 71);

    Theme {
        name: "VS Code Dark Modern".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(22, 22, 22),
        bg_base: bg,
        bg_surface: bg_secondary,
        bg_elevated: bg_active,
        bg_highlight: bg_highlight,
        markdown_code_bg: Color::Rgb(22, 22, 22),
        markdown_inline_code_bg: bg_secondary,

        text_bright: Color::Rgb(255, 255, 255),
        text_primary: fg,
        text_secondary: Color::Rgb(172, 178, 190),
        text_muted: Color::Rgb(130, 136, 148),
        text_faint: Color::Rgb(90, 95, 105),

        accent_primary: keyword,
        accent_secondary: variable,
        accent_success: type_color,
        accent_warning: func,
        accent_error: error,

        agent_claude: type_color,
        agent_codex: variable,
        agent_opencode: keyword,

        pr_open_bg: Color::Rgb(70, 180, 80),
        pr_merged_bg: Color::Rgb(130, 80, 200),
        pr_closed_bg: error,
        pr_draft_bg: Color::Rgb(130, 136, 148),
        pr_unknown_bg: bg_active,

        spinner_active: keyword,
        spinner_trail_1: Color::Rgb(70, 134, 190),
        spinner_trail_2: Color::Rgb(56, 113, 165),
        spinner_trail_3: Color::Rgb(44, 93, 140),
        spinner_trail_4: Color::Rgb(34, 75, 115),
        spinner_trail_5: Color::Rgb(26, 58, 92),
        spinner_inactive: Color::Rgb(130, 136, 148),

        border_default: bg_secondary,
        border_focused: keyword,
        border_dimmed: Color::Rgb(36, 40, 48),

        shine_edge: Color::Rgb(105, 110, 122),
        shine_mid: Color::Rgb(158, 163, 175),
        shine_center: fg,
        shine_peak: Color::Rgb(255, 255, 255),

        tool_block_bg: Color::Rgb(22, 22, 22),
        tool_comment: comment,
        tool_command: fg,
        tool_output: Color::Rgb(172, 178, 190),
        diff_add: Color::Rgb(70, 180, 80),
        diff_remove: error,
    }
}

/// Shades of Purple theme.
pub fn shades_of_purple() -> Theme {
    let background = Color::Rgb(45, 43, 85);
    let bg_light = Color::Rgb(57, 55, 105);
    let selection = Color::Rgb(59, 61, 113);
    let foreground = Color::Rgb(165, 153, 233);
    let comment = Color::Rgb(179, 98, 255);
    let red = Color::Rgb(251, 54, 64);
    let yellow = Color::Rgb(250, 208, 0);
    let green = Color::Rgb(165, 255, 144);
    let cyan = Color::Rgb(54, 249, 246);
    let pink = Color::Rgb(255, 98, 140);

    Theme {
        name: "Shades of Purple".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(30, 28, 65),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(72, 70, 130),
        markdown_code_bg: Color::Rgb(30, 28, 65),
        markdown_inline_code_bg: bg_light,

        text_bright: Color::Rgb(255, 255, 255),
        text_primary: foreground,
        text_secondary: Color::Rgb(140, 128, 200),
        text_muted: Color::Rgb(110, 100, 165),
        text_faint: Color::Rgb(75, 70, 120),

        accent_primary: comment,
        accent_secondary: pink,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: comment,
        agent_opencode: Color::Rgb(100, 160, 255),

        pr_open_bg: green,
        pr_merged_bg: comment,
        pr_closed_bg: red,
        pr_draft_bg: Color::Rgb(110, 100, 165),
        pr_unknown_bg: selection,

        spinner_active: comment,
        spinner_trail_1: Color::Rgb(155, 80, 230),
        spinner_trail_2: Color::Rgb(130, 65, 205),
        spinner_trail_3: Color::Rgb(108, 52, 180),
        spinner_trail_4: Color::Rgb(88, 42, 155),
        spinner_trail_5: Color::Rgb(70, 45, 130),
        spinner_inactive: Color::Rgb(110, 100, 165),

        border_default: bg_light,
        border_focused: comment,
        border_dimmed: Color::Rgb(40, 38, 80),

        shine_edge: Color::Rgb(110, 100, 165),
        shine_mid: foreground,
        shine_center: Color::Rgb(210, 200, 255),
        shine_peak: Color::Rgb(255, 255, 255),

        tool_block_bg: Color::Rgb(30, 28, 65),
        tool_comment: Color::Rgb(110, 100, 165),
        tool_command: foreground,
        tool_output: Color::Rgb(140, 128, 200),
        diff_add: green,
        diff_remove: red,
    }
}

/// Night Owl theme.
pub fn night_owl() -> Theme {
    let background = Color::Rgb(1, 22, 39);
    let bg_highlight = Color::Rgb(1, 31, 53);
    let bg_elevated = Color::Rgb(13, 43, 69);
    let bg_selection = Color::Rgb(28, 56, 80);
    let foreground = Color::Rgb(214, 222, 235);
    let comment = Color::Rgb(99, 119, 119);
    let red = Color::Rgb(239, 83, 80);
    let yellow = Color::Rgb(255, 203, 139);
    let green = Color::Rgb(173, 219, 103);
    let _cyan = Color::Rgb(128, 203, 196);
    let blue = Color::Rgb(130, 170, 255);
    let purple = Color::Rgb(199, 146, 234);
    let teal = Color::Rgb(127, 219, 202);

    Theme {
        name: "Night Owl".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(0, 15, 27),
        bg_base: background,
        bg_surface: bg_highlight,
        bg_elevated: bg_elevated,
        bg_highlight: bg_selection,
        markdown_code_bg: Color::Rgb(0, 15, 27),
        markdown_inline_code_bg: bg_highlight,

        text_bright: foreground,
        text_primary: Color::Rgb(195, 205, 220),
        text_secondary: Color::Rgb(150, 165, 185),
        text_muted: comment,
        text_faint: Color::Rgb(55, 75, 90),

        accent_primary: blue,
        accent_secondary: purple,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: teal,
        agent_codex: purple,
        agent_opencode: blue,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: bg_elevated,

        spinner_active: blue,
        spinner_trail_1: Color::Rgb(110, 145, 225),
        spinner_trail_2: Color::Rgb(90, 122, 200),
        spinner_trail_3: Color::Rgb(72, 100, 175),
        spinner_trail_4: Color::Rgb(56, 80, 150),
        spinner_trail_5: Color::Rgb(45, 80, 130),
        spinner_inactive: comment,

        border_default: bg_elevated,
        border_focused: blue,
        border_dimmed: bg_highlight,

        shine_edge: comment,
        shine_mid: Color::Rgb(150, 165, 185),
        shine_center: foreground,
        shine_peak: Color::Rgb(255, 255, 255),

        tool_block_bg: Color::Rgb(0, 15, 27),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(150, 165, 185),
        diff_add: green,
        diff_remove: red,
    }
}

/// Synthwave '84 theme.
pub fn synthwave() -> Theme {
    let background = Color::Rgb(38, 35, 53);
    let bg_light = Color::Rgb(52, 49, 72);
    let selection = Color::Rgb(65, 62, 90);
    let foreground = Color::Rgb(255, 255, 255);
    let comment = Color::Rgb(132, 139, 189);
    let red = Color::Rgb(254, 68, 80);
    let yellow = Color::Rgb(254, 222, 93);
    let green = Color::Rgb(114, 241, 184);
    let cyan = Color::Rgb(54, 249, 246);
    let purple = Color::Rgb(255, 42, 252);
    let pink = Color::Rgb(255, 126, 219);

    Theme {
        name: "Synthwave '84".to_string(),
        is_light: false,

        bg_terminal: Color::Rgb(26, 24, 38),
        bg_base: background,
        bg_surface: bg_light,
        bg_elevated: selection,
        bg_highlight: Color::Rgb(80, 77, 110),
        markdown_code_bg: Color::Rgb(26, 24, 38),
        markdown_inline_code_bg: bg_light,

        text_bright: foreground,
        text_primary: Color::Rgb(230, 228, 255),
        text_secondary: comment,
        text_muted: Color::Rgb(105, 112, 162),
        text_faint: Color::Rgb(70, 67, 105),

        accent_primary: cyan,
        accent_secondary: pink,
        accent_success: green,
        accent_warning: yellow,
        accent_error: red,

        agent_claude: cyan,
        agent_codex: purple,
        agent_opencode: pink,

        pr_open_bg: green,
        pr_merged_bg: purple,
        pr_closed_bg: red,
        pr_draft_bg: comment,
        pr_unknown_bg: selection,

        spinner_active: cyan,
        spinner_trail_1: Color::Rgb(40, 200, 200),
        spinner_trail_2: Color::Rgb(34, 168, 168),
        spinner_trail_3: Color::Rgb(28, 138, 138),
        spinner_trail_4: Color::Rgb(24, 110, 110),
        spinner_trail_5: Color::Rgb(20, 90, 90),
        spinner_inactive: comment,

        border_default: bg_light,
        border_focused: cyan,
        border_dimmed: Color::Rgb(40, 38, 58),

        shine_edge: comment,
        shine_mid: Color::Rgb(180, 175, 230),
        shine_center: Color::Rgb(225, 220, 255),
        shine_peak: foreground,

        tool_block_bg: Color::Rgb(26, 24, 38),
        tool_comment: comment,
        tool_command: foreground,
        tool_output: Color::Rgb(180, 175, 230),
        diff_add: green,
        diff_remove: red,
    }
}
