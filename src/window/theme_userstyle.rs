use super::*;
use sourceview5::prelude::*;

const USERSTYLE_PRESET_NAMES: [&str; 12] = [
    "Custom",
    "Cardthropic",
    "Cardthropic Night",
    "Cardthropic Midnight",
    "Arcade",
    "Glass",
    "Neon",
    "Noir",
    "Forest",
    "CRT",
    "Terminal",
    "Minimal Mono",
];

const USERSTYLE_TEMPLATE_ARCADE: &str = r#"/* Arcade */
window,
window background,
box,
label {
  color: #f4fbff;
}

.board-background {
  background-image:
    repeating-linear-gradient(
      45deg,
      rgba(0, 0, 0, 0.26),
      rgba(0, 0, 0, 0.26) 12px,
      rgba(255, 255, 255, 0.05) 12px,
      rgba(255, 255, 255, 0.05) 24px
    ),
    linear-gradient(160deg, #0f2f55, #133c2d);
  border: 3px solid rgba(255, 255, 255, 0.42);
  box-shadow: 0 16px 42px rgba(0, 0, 0, 0.50);
}

headerbar,
popover,
menu,
frame {
  background: rgba(10, 18, 34, 0.56);
}

button {
  color: #f4fbff;
  border-radius: 12px;
  border: 2px solid rgba(255, 255, 255, 0.30);
  background-image: linear-gradient(180deg, #244a73, #132a45);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.42);
}

button:hover {
  border-color: rgba(255, 255, 255, 0.60);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f8fcff;
  background: rgba(8, 15, 30, 0.58);
  border: 1px solid rgba(255, 255, 255, 0.42);
}

.status-line {
  color: #e7f7ff;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(226, 243, 255, 0.88);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.96),
    inset 0 0 0 7px rgba(250, 204, 21, 0.88);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(250, 204, 21, 0.88);
  background-color: rgba(250, 204, 21, 0.12);
}

.keyboard-focus-card {
  box-shadow:
    inset 0 0 0 2px #0d1020,
    inset 0 0 0 6px #ffffff,
    inset 0 0 0 9px #3dd9ff;
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.28);
  background-color: rgba(255, 255, 255, 0.05);
}

.slot-emoji {
  opacity: 0.86;
}
"#;

const USERSTYLE_TEMPLATE_GLASS: &str = r#"/* Glass */
window,
window background,
box,
label {
  color: #eef5ff;
}

.board-background {
  background-image:
    linear-gradient(140deg, rgba(16, 38, 76, 0.90), rgba(49, 26, 72, 0.92)),
    radial-gradient(circle at 20% 20%, rgba(255, 255, 255, 0.10), transparent 45%),
    radial-gradient(circle at 80% 75%, rgba(56, 189, 248, 0.12), transparent 38%);
  border: 2px solid rgba(255, 255, 255, 0.30);
  box-shadow:
    0 18px 44px rgba(0, 0, 0, 0.46),
    inset 0 0 0 1px rgba(255, 255, 255, 0.16);
}

headerbar,
popover,
frame {
  background: rgba(18, 24, 45, 0.50);
}

button {
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.30);
  background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.22), rgba(255, 255, 255, 0.10));
}

entry,
combobox,
dropdown,
popover entry {
  background: rgba(10, 16, 30, 0.48);
  border: 1px solid rgba(255, 255, 255, 0.36);
}

.status-line {
  color: #eaf6ff;
}

.stats-line,
.dim-label {
  color: rgba(224, 238, 255, 0.84);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.96),
    inset 0 0 0 7px rgba(56, 189, 248, 0.74);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(56, 189, 248, 0.74);
  background-color: rgba(56, 189, 248, 0.11);
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.24);
  background-color: rgba(255, 255, 255, 0.04);
}

.slot-emoji {
  opacity: 0.80;
}
"#;

const USERSTYLE_TEMPLATE_NEON: &str = r#"/* Neon */
window,
label {
  color: #fff4ff;
}

.board-background {
  background-image:
    radial-gradient(circle at 15% 10%, rgba(255, 0, 128, 0.32), transparent 40%),
    radial-gradient(circle at 85% 85%, rgba(0, 240, 255, 0.30), transparent 42%),
    linear-gradient(145deg, #3d1a6b, #0b3a6b);
  border: 2px solid rgba(255, 128, 230, 0.55);
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.22),
    0 0 26px rgba(255, 0, 170, 0.45),
    0 0 52px rgba(0, 210, 255, 0.32);
}

headerbar,
popover,
frame {
  background: rgba(22, 10, 38, 0.56);
}

button {
  color: #fff8ff;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.28);
  background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.20), rgba(255, 255, 255, 0.08));
}

entry,
combobox,
dropdown,
popover entry {
  background: rgba(14, 18, 32, 0.60);
  border: 1px solid rgba(255, 255, 255, 0.35);
}

.status-line {
  color: #ffe7fa;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(255, 226, 250, 0.84);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.96),
    inset 0 0 0 7px rgba(236, 72, 153, 0.86);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(236, 72, 153, 0.86);
  background-color: rgba(236, 72, 153, 0.14);
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.26);
  background-color: rgba(255, 255, 255, 0.05);
}

.slot-emoji {
  opacity: 0.90;
}
"#;

const USERSTYLE_TEMPLATE_NOIR: &str = r#"/* Noir */
window,
window background,
box,
label {
  color: #e8e8e8;
}

.board-background {
  background-image:
    linear-gradient(180deg, #1f1f1f, #111111),
    repeating-linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.03),
      rgba(255, 255, 255, 0.03) 2px,
      transparent 2px,
      transparent 6px
    );
  border: 2px solid rgba(255, 255, 255, 0.16);
  box-shadow: 0 14px 36px rgba(0, 0, 0, 0.60);
}

headerbar,
popover,
frame {
  background: rgba(14, 14, 14, 0.66);
}

button {
  color: #f2f2f2;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.25);
  background-image: linear-gradient(180deg, #2e2e2e, #202020);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f4f4f4;
  background: rgba(12, 12, 12, 0.70);
  border: 1px solid rgba(255, 255, 255, 0.24);
}

.status-line {
  color: #fafafa;
}

.stats-line,
.dim-label {
  color: rgba(244, 244, 244, 0.80);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.94),
    inset 0 0 0 7px rgba(180, 180, 180, 0.90);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(180, 180, 180, 0.90);
  background-color: rgba(220, 220, 220, 0.10);
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.20);
  background-color: rgba(255, 255, 255, 0.02);
}

.slot-emoji {
  opacity: 0.72;
}
"#;

const USERSTYLE_TEMPLATE_FOREST: &str = r#"/* Forest */
window,
window background,
box,
label {
  color: #ebf6eb;
}

.board-background {
  background-image:
    radial-gradient(circle at 18% 15%, rgba(191, 255, 173, 0.12), transparent 40%),
    radial-gradient(circle at 82% 80%, rgba(54, 255, 188, 0.11), transparent 40%),
    linear-gradient(150deg, #183a24, #1d4f34);
  border: 2px solid rgba(205, 255, 227, 0.30);
  box-shadow: 0 14px 36px rgba(0, 0, 0, 0.45);
}

headerbar,
popover,
frame {
  background: rgba(17, 42, 28, 0.55);
}

button {
  color: #f2fff4;
  border-radius: 12px;
  border: 1px solid rgba(210, 255, 225, 0.30);
  background-image: linear-gradient(180deg, #21563a, #1a402d);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f2fff4;
  background: rgba(12, 32, 22, 0.60);
  border: 1px solid rgba(209, 255, 223, 0.30);
}

.status-line {
  color: #e8ffe9;
}

.stats-line,
.dim-label {
  color: rgba(225, 255, 231, 0.84);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.94),
    inset 0 0 0 7px rgba(74, 222, 128, 0.84);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(74, 222, 128, 0.84);
  background-color: rgba(74, 222, 128, 0.12);
}

.card-slot {
  border: 1px solid rgba(216, 255, 229, 0.24);
  background-color: rgba(255, 255, 255, 0.03);
}

.slot-emoji {
  opacity: 0.84;
}
"#;

const USERSTYLE_TEMPLATE_CARDTHROPIC: &str = r#"/* Cardthropic Signature */
window,
window background,
box,
label {
  color: #f7fbff;
}

.board-background {
  background-image:
    radial-gradient(circle at 14% 14%, rgba(255, 86, 118, 0.16), transparent 34%),
    radial-gradient(circle at 82% 78%, rgba(255, 197, 88, 0.12), transparent 35%),
    linear-gradient(155deg, #1d2846 0%, #2b2152 42%, #182f3f 100%);
  border: 2px solid rgba(255, 255, 255, 0.36);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.14),
    0 18px 40px rgba(0, 0, 0, 0.45);
}

headerbar,
popover,
frame {
  background: rgba(20, 25, 45, 0.68);
}

button {
  color: #f8fbff;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.34);
  background-image: linear-gradient(180deg, #3a4f87, #273762);
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.32);
}

button:hover {
  border-color: rgba(255, 255, 255, 0.58);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f5f9ff;
  background: rgba(15, 20, 36, 0.72);
  border: 1px solid rgba(255, 255, 255, 0.40);
}

.status-line {
  color: #ffffff;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(232, 241, 255, 0.90);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.98),
    inset 0 0 0 7px rgba(255, 108, 138, 0.88);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(255, 108, 138, 0.88);
  background-color: rgba(255, 108, 138, 0.14);
}

.keyboard-focus-card {
  box-shadow:
    inset 0 0 0 2px #0d1020,
    inset 0 0 0 6px #ffffff,
    inset 0 0 0 9px #f8bd55;
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.28);
  background-color: rgba(255, 255, 255, 0.04);
}

.slot-emoji {
  opacity: 0.86;
}
"#;

const USERSTYLE_TEMPLATE_CARDTHROPIC_NIGHT: &str = r#"/* Cardthropic Night */
window,
window background,
box,
label {
  color: #f5f9ff;
}

.board-background {
  background-image:
    radial-gradient(circle at 12% 15%, rgba(110, 178, 255, 0.12), transparent 36%),
    radial-gradient(circle at 84% 80%, rgba(255, 132, 188, 0.10), transparent 38%),
    linear-gradient(155deg, #1a2540 0%, #27214d 48%, #173042 100%);
  border: 2px solid rgba(232, 242, 255, 0.34);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.12),
    0 16px 36px rgba(0, 0, 0, 0.44);
}

headerbar,
popover,
frame {
  background: rgba(22, 28, 50, 0.70);
}

button {
  color: #f7fbff;
  border-radius: 12px;
  border: 1px solid rgba(232, 242, 255, 0.30);
  background-image: linear-gradient(180deg, #455c95, #2e4070);
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.30);
}

button:hover {
  border-color: rgba(255, 255, 255, 0.52);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f2f7ff;
  background: rgba(15, 22, 40, 0.76);
  border: 1px solid rgba(225, 237, 255, 0.36);
}

.status-line {
  color: #ffffff;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(230, 240, 255, 0.88);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.97),
    inset 0 0 0 7px rgba(124, 180, 255, 0.88);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(124, 180, 255, 0.88);
  background-color: rgba(124, 180, 255, 0.14);
}

.keyboard-focus-card {
  box-shadow:
    inset 0 0 0 2px #0d1020,
    inset 0 0 0 6px #ffffff,
    inset 0 0 0 9px #7cb4ff;
}

.card-slot {
  border: 1px solid rgba(232, 242, 255, 0.26);
  background-color: rgba(255, 255, 255, 0.04);
}

.slot-emoji {
  opacity: 0.84;
}
"#;

const USERSTYLE_TEMPLATE_CARDTHROPIC_MIDNIGHT: &str = r#"/* Cardthropic Midnight */
window,
window background,
box,
label {
  color: #f8fbff;
}

.board-background {
  background-image:
    radial-gradient(circle at 18% 12%, rgba(82, 130, 255, 0.10), transparent 34%),
    radial-gradient(circle at 82% 82%, rgba(255, 94, 140, 0.08), transparent 36%),
    linear-gradient(160deg, #11182d 0%, #171a34 46%, #132033 100%);
  border: 2px solid rgba(225, 236, 255, 0.28);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.10),
    0 18px 42px rgba(0, 0, 0, 0.56);
}

headerbar,
popover,
frame {
  background: rgba(14, 18, 34, 0.80);
}

button {
  color: #f9fbff;
  border-radius: 12px;
  border: 1px solid rgba(225, 236, 255, 0.26);
  background-image: linear-gradient(180deg, #34466f, #253353);
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.38);
}

button:hover {
  border-color: rgba(255, 255, 255, 0.48);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f5f8ff;
  background: rgba(9, 14, 28, 0.84);
  border: 1px solid rgba(221, 233, 255, 0.34);
}

.status-line {
  color: #ffffff;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(226, 238, 255, 0.86);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.97),
    inset 0 0 0 7px rgba(130, 153, 255, 0.88);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(130, 153, 255, 0.88);
  background-color: rgba(130, 153, 255, 0.14);
}

.keyboard-focus-card {
  box-shadow:
    inset 0 0 0 2px #0d1020,
    inset 0 0 0 6px #ffffff,
    inset 0 0 0 9px #8299ff;
}

.card-slot {
  border: 1px solid rgba(225, 236, 255, 0.24);
  background-color: rgba(255, 255, 255, 0.03);
}

.slot-emoji {
  opacity: 0.82;
}
"#;

const USERSTYLE_TEMPLATE_CRT: &str = r#"/* CRT */
window,
window background,
box,
label {
  color: #d9ffd0;
}

.board-background {
  background-image:
    repeating-linear-gradient(
      0deg,
      rgba(0, 255, 120, 0.08),
      rgba(0, 255, 120, 0.08) 1px,
      rgba(0, 0, 0, 0.0) 1px,
      rgba(0, 0, 0, 0.0) 3px
    ),
    radial-gradient(circle at 50% 30%, rgba(0, 255, 136, 0.10), transparent 65%),
    linear-gradient(180deg, #04120a, #021008);
  border: 2px solid rgba(85, 255, 170, 0.40);
  box-shadow:
    inset 0 0 24px rgba(0, 255, 120, 0.16),
    0 0 22px rgba(0, 255, 120, 0.25);
}

headerbar,
popover,
frame {
  background: rgba(2, 18, 10, 0.72);
}

button {
  color: #dcffd6;
  border-radius: 8px;
  border: 1px solid rgba(99, 255, 169, 0.45);
  background-image: linear-gradient(180deg, #08301b, #052313);
}

entry,
combobox,
dropdown,
popover entry {
  color: #d9ffd0;
  background: rgba(2, 15, 9, 0.86);
  border: 1px solid rgba(99, 255, 169, 0.38);
}

.status-line {
  color: #deffda;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(211, 255, 203, 0.82);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(215, 255, 210, 0.95),
    inset 0 0 0 7px rgba(16, 255, 134, 0.90);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(16, 255, 134, 0.90);
  background-color: rgba(16, 255, 134, 0.12);
}

.card-slot {
  border: 1px solid rgba(90, 255, 164, 0.30);
  background-color: rgba(16, 255, 134, 0.04);
}

.slot-emoji {
  opacity: 0.84;
}
"#;

const USERSTYLE_TEMPLATE_TERMINAL: &str = r#"/* Terminal */
window,
window background,
box,
label {
  color: #f4f4f4;
}

.board-background {
  background-image:
    linear-gradient(180deg, #1f2329, #15181d),
    repeating-linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.03),
      rgba(255, 255, 255, 0.03) 1px,
      transparent 1px,
      transparent 8px
    );
  border: 2px solid rgba(255, 255, 255, 0.18);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.50);
}

headerbar,
popover,
frame {
  background: rgba(20, 22, 28, 0.76);
}

button {
  color: #f0f0f0;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background-image: linear-gradient(180deg, #303642, #262b35);
}

entry,
combobox,
dropdown,
popover entry {
  color: #f0f0f0;
  background: rgba(23, 26, 33, 0.88);
  border: 1px solid rgba(255, 255, 255, 0.24);
}

.status-line {
  color: #ffffff;
}

.stats-line,
.dim-label {
  color: rgba(235, 235, 235, 0.78);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.95),
    inset 0 0 0 7px rgba(125, 211, 252, 0.88);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(125, 211, 252, 0.88);
  background-color: rgba(125, 211, 252, 0.12);
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.20);
  background-color: rgba(255, 255, 255, 0.03);
}

.slot-emoji {
  opacity: 0.76;
}
"#;

const USERSTYLE_TEMPLATE_MINIMAL_MONO: &str = r#"/* Minimal Mono */
window,
window background,
box,
label {
  color: #ececec;
}

.board-background {
  background: #2b3242;
  border: 1px solid rgba(255, 255, 255, 0.20);
  box-shadow: none;
}

headerbar,
popover,
frame {
  background: rgba(40, 46, 60, 0.82);
}

button {
  color: #f2f2f2;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background: rgba(255, 255, 255, 0.08);
}

entry,
combobox,
dropdown,
popover entry {
  color: #efefef;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.24);
}

.status-line {
  color: #f4f4f4;
}

.stats-line,
.dim-label {
  color: rgba(236, 236, 236, 0.78);
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 3px rgba(255, 255, 255, 0.94),
    inset 0 0 0 7px rgba(196, 196, 196, 0.84);
}

.tableau-selected-empty,
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(196, 196, 196, 0.84);
  background-color: rgba(196, 196, 196, 0.10);
}

.card-slot {
  border: 1px solid rgba(255, 255, 255, 0.20);
  background-color: rgba(255, 255, 255, 0.02);
}

.slot-emoji {
  opacity: 0.68;
}
"#;

impl CardthropicWindow {
    pub(super) fn userstyle_preset_names() -> &'static [&'static str] {
        &USERSTYLE_PRESET_NAMES
    }

    pub(super) fn default_userstyle_css() -> &'static str {
        USERSTYLE_TEMPLATE_CARDTHROPIC
    }

    pub(super) fn userstyle_css_for_preset(index: u32) -> Option<&'static str> {
        match index {
            1 => Some(USERSTYLE_TEMPLATE_CARDTHROPIC),
            2 => Some(USERSTYLE_TEMPLATE_CARDTHROPIC_NIGHT),
            3 => Some(USERSTYLE_TEMPLATE_CARDTHROPIC_MIDNIGHT),
            4 => Some(USERSTYLE_TEMPLATE_ARCADE),
            5 => Some(USERSTYLE_TEMPLATE_GLASS),
            6 => Some(USERSTYLE_TEMPLATE_NEON),
            7 => Some(USERSTYLE_TEMPLATE_NOIR),
            8 => Some(USERSTYLE_TEMPLATE_FOREST),
            9 => Some(USERSTYLE_TEMPLATE_CRT),
            10 => Some(USERSTYLE_TEMPLATE_TERMINAL),
            11 => Some(USERSTYLE_TEMPLATE_MINIMAL_MONO),
            _ => None,
        }
    }

    pub(super) fn userstyle_preset_for_css(css: &str) -> u32 {
        if css == USERSTYLE_TEMPLATE_CARDTHROPIC {
            1
        } else if css == USERSTYLE_TEMPLATE_CARDTHROPIC_NIGHT {
            2
        } else if css == USERSTYLE_TEMPLATE_CARDTHROPIC_MIDNIGHT {
            3
        } else if css == USERSTYLE_TEMPLATE_ARCADE {
            4
        } else if css == USERSTYLE_TEMPLATE_GLASS {
            5
        } else if css == USERSTYLE_TEMPLATE_NEON {
            6
        } else if css == USERSTYLE_TEMPLATE_NOIR {
            7
        } else if css == USERSTYLE_TEMPLATE_FOREST {
            8
        } else if css == USERSTYLE_TEMPLATE_CRT {
            9
        } else if css == USERSTYLE_TEMPLATE_TERMINAL {
            10
        } else if css == USERSTYLE_TEMPLATE_MINIMAL_MONO {
            11
        } else {
            0
        }
    }

    pub(super) fn apply_userstyle_preset(&self, index: u32, persist: bool) {
        if let Some(css) = Self::userstyle_css_for_preset(index) {
            self.apply_custom_userstyle(css, persist);
        }
    }

    #[allow(deprecated)]
    fn validate_userstyle_css(css: &str) -> Result<(), String> {
        if css.trim().is_empty() {
            return Ok(());
        }

        let provider = gtk::CssProvider::new();
        let had_error = Rc::new(Cell::new(false));
        let first_error = Rc::new(RefCell::new(None::<String>));
        provider.connect_parsing_error(glib::clone!(
            #[strong]
            had_error,
            #[strong]
            first_error,
            move |_, _, error| {
                had_error.set(true);
                if first_error.borrow().is_none() {
                    *first_error.borrow_mut() = Some(error.to_string());
                }
            }
        ));
        provider.load_from_string(css);

        if had_error.get() {
            Err(first_error
                .borrow()
                .clone()
                .unwrap_or_else(|| "CSS parser reported an error.".to_string()))
        } else {
            Ok(())
        }
    }

    pub(super) fn apply_custom_userstyle(&self, css: &str, persist: bool) {
        let imp = self.imp();

        let existing_provider = imp.custom_userstyle_provider.borrow().clone();
        let provider = if let Some(provider) = existing_provider {
            provider
        } else {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &self.display(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
            *imp.custom_userstyle_provider.borrow_mut() = Some(provider.clone());
            provider
        };

        provider.load_from_string(css);
        *imp.custom_userstyle_css.borrow_mut() = css.to_string();

        if persist {
            let settings = imp.settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_CUSTOM_USERSTYLE_CSS, css);
            }
        }
    }

    pub(super) fn open_custom_userstyle_dialog(&self) {
        if let Some(existing) = self.imp().custom_userstyle_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let dialog = gtk::Window::builder()
            .title("Custom CSS Userstyle")
            .resizable(true)
            .default_width(760)
            .default_height(560)
            .build();
        dialog.set_transient_for(None::<&gtk::Window>);
        dialog.set_hide_on_close(true);
        dialog.set_destroy_with_parent(false);

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(glib::clone!(
            #[weak]
            dialog,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    dialog.close();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        dialog.add_controller(key_controller);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let title = gtk::Label::new(Some("Custom CSS Userstyle"));
        title.set_xalign(0.0);
        title.add_css_class("title-4");
        root.append(&title);

        let subtitle = gtk::Label::new(Some(
            "Opinionated mode: start from a preset, preview fast, then save. Applies over board/theme styling.",
        ));
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.add_css_class("dim-label");
        root.append(&subtitle);

        let presets_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let presets_label = gtk::Label::new(Some("Preset"));
        presets_label.set_xalign(0.0);
        presets_label.add_css_class("dim-label");
        let presets_dropdown = gtk::DropDown::from_strings(Self::userstyle_preset_names());
        presets_dropdown.set_selected(Self::userstyle_preset_for_css(
            &self.imp().custom_userstyle_css.borrow(),
        ));
        presets_dropdown.set_hexpand(true);
        presets_row.append(&presets_label);
        presets_row.append(&presets_dropdown);
        root.append(&presets_row);

        let font_sizes: [u32; 10] = [8, 9, 10, 11, 12, 13, 14, 16, 18, 20];
        let font_size_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let font_size_label = gtk::Label::new(Some("Font Size"));
        font_size_label.set_xalign(0.0);
        font_size_label.add_css_class("dim-label");
        let font_size_dropdown = gtk::DropDown::from_strings(&[
            "8", "9", "10", "11", "12", "13", "14", "16", "18", "20",
        ]);
        font_size_dropdown.set_hexpand(true);
        font_size_dropdown.set_selected(5);
        font_size_row.append(&font_size_label);
        font_size_row.append(&font_size_dropdown);
        root.append(&font_size_row);

        let scrolled = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .min_content_height(280)
            .build();
        let language_manager = sourceview5::LanguageManager::new();
        let language = language_manager.language("css");
        let buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
        if let Some(language) = language {
            buffer.set_language(Some(&language));
        }
        let scheme_manager = sourceview5::StyleSchemeManager::new();
        if let Some(scheme) = scheme_manager
            .scheme("Adwaita-dark")
            .or_else(|| scheme_manager.scheme("classic-dark"))
            .or_else(|| scheme_manager.scheme("oblivion"))
        {
            buffer.set_style_scheme(Some(&scheme));
        }
        buffer.set_highlight_syntax(true);
        buffer.set_highlight_matching_brackets(true);
        let text_view = sourceview5::View::with_buffer(&buffer);
        text_view.set_monospace(true);
        text_view.set_show_line_numbers(true);
        text_view.set_tab_width(2);
        text_view.set_insert_spaces_instead_of_tabs(true);
        text_view.set_auto_indent(true);
        text_view.set_wrap_mode(gtk::WrapMode::None);
        text_view.add_css_class("code");
        text_view.add_css_class("userstyle-editor-view");
        buffer.set_text(&self.imp().custom_userstyle_css.borrow());
        scrolled.set_child(Some(&text_view));
        root.append(&scrolled);

        let editor_font_provider = gtk::CssProvider::new();
        gtk::style_context_add_provider_for_display(
            &self.display(),
            &editor_font_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 5,
        );
        editor_font_provider.load_from_string(".userstyle-editor-view { font-size: 13pt; }");

        let hint = gtk::Label::new(Some(
            "Tip: target classes like .board-background, .tableau-selected-card, .waste-selected-card.",
        ));
        hint.set_xalign(0.0);
        hint.add_css_class("dim-label");
        root.append(&hint);

        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let live_preview = gtk::CheckButton::with_label("Live preview");
        live_preview.set_active(true);
        let status = gtk::Label::new(Some("Ready"));
        status.set_xalign(0.0);
        status.add_css_class("dim-label");
        status.set_hexpand(true);
        controls.append(&live_preview);
        controls.append(&status);
        root.append(&controls);

        let diagnostics = gtk::Label::new(Some("CSS diagnostics: Ready"));
        diagnostics.set_xalign(0.0);
        diagnostics.add_css_class("dim-label");
        root.append(&diagnostics);

        let clipboard_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let copy_css = gtk::Button::with_label("Copy CSS");
        copy_css.add_css_class("flat");
        let paste_css = gtk::Button::with_label("Paste CSS");
        paste_css.add_css_class("flat");
        let copy_preset = gtk::Button::with_label("Copy Preset + CSS");
        copy_preset.add_css_class("flat");
        clipboard_row.append(&copy_css);
        clipboard_row.append(&paste_css);
        clipboard_row.append(&copy_preset);
        root.append(&clipboard_row);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let reset = gtk::Button::with_label("Reset");
        reset.add_css_class("flat");
        let apply = gtk::Button::with_label("Apply Preview");
        apply.add_css_class("flat");
        let save = gtk::Button::with_label("Save");
        save.add_css_class("suggested-action");
        let close = gtk::Button::with_label("Close");
        actions.append(&reset);
        actions.append(&apply);
        actions.append(&save);
        actions.append(&close);
        root.append(&actions);

        copy_css.connect_clicked(glib::clone!(
            #[weak]
            buffer,
            #[weak]
            status,
            move |_| {
                if let Some(display) = gdk::Display::default() {
                    let clipboard = display.clipboard();
                    let text = buffer
                        .text(&buffer.start_iter(), &buffer.end_iter(), true)
                        .to_string();
                    clipboard.set_text(&text);
                    status.set_label("Copied CSS to clipboard");
                } else {
                    status.set_label("Clipboard unavailable");
                }
            }
        ));

        paste_css.connect_clicked(glib::clone!(
            #[weak]
            buffer,
            #[weak]
            status,
            #[weak]
            diagnostics,
            move |_| {
                if let Some(display) = gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.read_text_async(
                        None::<&gio::Cancellable>,
                        glib::clone!(
                            #[weak]
                            buffer,
                            #[weak]
                            status,
                            #[weak]
                            diagnostics,
                            move |result| match result {
                                Ok(Some(text)) => {
                                    let text = text.to_string();
                                    buffer.set_text(&text);
                                    status.set_label("Pasted CSS from clipboard");
                                    match Self::validate_userstyle_css(&text) {
                                        Ok(_) => {
                                            diagnostics.set_label("CSS diagnostics: Looks valid.");
                                            diagnostics.remove_css_class("error");
                                        }
                                        Err(err) => {
                                            diagnostics
                                                .set_label(&format!("CSS diagnostics: {err}"));
                                            diagnostics.add_css_class("error");
                                        }
                                    }
                                }
                                Ok(None) => status.set_label("Clipboard has no text"),
                                Err(_) => status.set_label("Failed to read clipboard text"),
                            }
                        ),
                    );
                } else {
                    status.set_label("Clipboard unavailable");
                }
            }
        ));

        copy_preset.connect_clicked(glib::clone!(
            #[weak]
            buffer,
            #[weak]
            presets_dropdown,
            #[weak]
            status,
            move |_| {
                if let Some(display) = gdk::Display::default() {
                    let clipboard = display.clipboard();
                    let css = buffer
                        .text(&buffer.start_iter(), &buffer.end_iter(), true)
                        .to_string();
                    let preset_idx = presets_dropdown.selected() as usize;
                    let preset_name = Self::userstyle_preset_names()
                        .get(preset_idx)
                        .copied()
                        .unwrap_or("Custom");
                    let payload = format!("/* Preset: {preset_name} */\n{css}");
                    clipboard.set_text(&payload);
                    status.set_label("Copied preset name + CSS to clipboard");
                } else {
                    status.set_label("Clipboard unavailable");
                }
            }
        ));

        font_size_dropdown.connect_selected_notify(glib::clone!(
            #[strong]
            editor_font_provider,
            move |dropdown| {
                let selected = dropdown.selected() as usize;
                let size = *font_sizes.get(selected).unwrap_or(&13);
                editor_font_provider.load_from_string(&format!(
                    ".userstyle-editor-view {{ font-size: {}pt; }}",
                    size
                ));
            }
        ));

        let shortcut_controller = gtk::EventControllerKey::new();
        shortcut_controller.connect_key_pressed(glib::clone!(
            #[weak]
            copy_css,
            #[weak]
            paste_css,
            #[weak]
            copy_preset,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, state| {
                let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
                let shift = state.contains(gdk::ModifierType::SHIFT_MASK);

                if ctrl && shift && matches!(key, gdk::Key::c | gdk::Key::C) {
                    copy_preset.emit_clicked();
                    return glib::Propagation::Stop;
                }
                if ctrl && !shift && matches!(key, gdk::Key::c | gdk::Key::C) {
                    copy_css.emit_clicked();
                    return glib::Propagation::Stop;
                }
                if ctrl && !shift && matches!(key, gdk::Key::v | gdk::Key::V) {
                    paste_css.emit_clicked();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        dialog.add_controller(shortcut_controller);

        presets_dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            move |dropdown| {
                if let Some(css) = Self::userstyle_css_for_preset(dropdown.selected()) {
                    buffer.set_text(css);
                    // Preset picker should immediately preview and persist as a preset choice.
                    window.apply_custom_userstyle(css, true);
                }
            }
        ));

        buffer.connect_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            live_preview,
            #[weak]
            status,
            #[weak]
            diagnostics,
            move |buf| {
                let text = buf
                    .text(&buf.start_iter(), &buf.end_iter(), true)
                    .to_string();
                match Self::validate_userstyle_css(&text) {
                    Ok(_) => {
                        diagnostics.set_label("CSS diagnostics: Looks valid.");
                        diagnostics.remove_css_class("error");
                    }
                    Err(err) => {
                        diagnostics.set_label(&format!("CSS diagnostics: {err}"));
                        diagnostics.add_css_class("error");
                    }
                }

                if live_preview.is_active() {
                    window.apply_custom_userstyle(&text, false);
                    status.set_label("Live preview applied");
                }
            }
        ));

        apply.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            status,
            #[weak]
            diagnostics,
            move |_| {
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                window.apply_custom_userstyle(&text, false);
                status.set_label("Preview applied");
                match Self::validate_userstyle_css(&text) {
                    Ok(_) => {
                        diagnostics.set_label("CSS diagnostics: Looks valid.");
                        diagnostics.remove_css_class("error");
                    }
                    Err(err) => {
                        diagnostics.set_label(&format!("CSS diagnostics: {err}"));
                        diagnostics.add_css_class("error");
                    }
                }
            }
        ));

        save.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            status,
            #[weak]
            diagnostics,
            move |_| {
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                window.apply_custom_userstyle(&text, true);
                status.set_label("Saved to preferences");
                match Self::validate_userstyle_css(&text) {
                    Ok(_) => {
                        diagnostics.set_label("CSS diagnostics: Looks valid.");
                        diagnostics.remove_css_class("error");
                    }
                    Err(err) => {
                        diagnostics.set_label(&format!("CSS diagnostics: {err}"));
                        diagnostics.add_css_class("error");
                    }
                }
            }
        ));

        reset.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            presets_dropdown,
            #[weak]
            status,
            #[weak]
            diagnostics,
            move |_| {
                buffer.set_text(Self::default_userstyle_css());
                window.apply_custom_userstyle(Self::default_userstyle_css(), true);
                status.set_label("Reset to default Cardthropic CSS");
                diagnostics.set_label("CSS diagnostics: Ready");
                diagnostics.remove_css_class("error");
                presets_dropdown
                    .set_selected(Self::userstyle_preset_for_css(Self::default_userstyle_css()));
            }
        ));

        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));

        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                *window.imp().custom_userstyle_dialog.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));

        dialog.set_child(Some(&root));
        *self.imp().custom_userstyle_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }
}
