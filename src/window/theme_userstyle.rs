use super::*;
use sourceview5::prelude::*;

const USERSTYLE_PRESET_NAMES: [&str; 21] = [
    "Custom",
    "System",
    "Light Mode",
    "Dark Mode",
    "Cardthropic",
    "Garnet",
    "Amethyst",
    "Aquamarine",
    "Diamond",
    "Emerald",
    "Pearl",
    "Ruby",
    "Peridot",
    "Sapphire",
    "Opal",
    "Topaz",
    "Turquoise",
    "Moonstone",
    "Citrine",
    "CRT",
    "Magma",
];

const USERSTYLE_TEMPLATE_SYSTEM: &str = r#"/* System
No custom CSS overrides.
*/"#;

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

const USERSTYLE_TEMPLATE_NEON: &str = r#"/* Cardthropic */
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

const USERSTYLE_TEMPLATE_TERMINAL: &str = r#"/* Dark Mode */
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

const USERSTYLE_TEMPLATE_LIGHT_MODE: &str = r#"/* Light Mode */
window,
window background {
  background: #ffffff;
  color: #111111;
}

.board-background {
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.10);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.06);
}

box,
frame {
  color: #111111;
}

headerbar {
  background: #f6f6f7;
  color: #111111;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}

popover,
menu,
popover.background,
menu.background {
  background: #ffffff;
  color: #111111;
  border: 1px solid rgba(0, 0, 0, 0.12);
}

popover contents,
menu contents {
  background: #ffffff;
  color: #111111;
}

modelbutton,
menuitem,
menu item,
popover label {
  color: #111111;
}

entry,
combobox,
dropdown,
popover entry {
  color: #111111;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.16);
}

button {
  color: #111111;
  font-weight: 600;
  border-radius: 11px;
  border: 1px solid rgba(0, 0, 0, 0.14);
  background-image: linear-gradient(180deg, #ffffff, #f1f1f3);
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.06),
    0 4px 10px rgba(0, 0, 0, 0.05);
}

button:hover {
  border-color: rgba(0, 0, 0, 0.22);
  background-image: linear-gradient(180deg, #ffffff, #ececef);
}

button:active {
  background-image: linear-gradient(180deg, #ececef, #ffffff);
}

.hud-toggle:checked {
  color: #005ecb;
  border-color: rgba(0, 94, 203, 0.40);
  background-image: linear-gradient(180deg, #eaf3ff, #dfeeff);
  box-shadow: inset 0 0 0 1px rgba(0, 94, 203, 0.20);
}

.status-line {
  color: #111111;
  font-weight: 700;
}

.stats-line,
.dim-label {
  color: rgba(0, 0, 0, 0.62);
}

.card-slot {
  border: 1px solid rgba(0, 0, 0, 0.20);
  background-color: #ffffff;
}

.tableau-selected-card,
.waste-selected-card {
  box-shadow:
    inset 0 0 0 2px rgba(0, 122, 255, 0.85),
    0 0 10px rgba(0, 122, 255, 0.18);
  background-color: rgba(0, 122, 255, 0.07);
}

.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(0, 122, 255, 0.60);
  background-color: rgba(0, 122, 255, 0.10);
}

.keyboard-focus-card {
  box-shadow:
    inset 0 0 0 2px #ffffff,
    inset 0 0 0 5px #101010,
    inset 0 0 0 8px #007aff;
}

.slot-emoji {
  opacity: 0.50;
}
"#;

const USERSTYLE_TEMPLATE_MAGMA: &str = r#"/* Magma Core */
window, window background { background: #1a0500; }

.board-background {
    background-image: 
        radial-gradient(circle at 50% 50%, rgba(255, 69, 0, 0.15), transparent 70%),
        repeating-linear-gradient(
            45deg,
            #2b0a00 0px,
            #2b0a00 2px,
            #1a0500 2px,
            #1a0500 20px
        ),
        linear-gradient(180deg, #3d0f00, #1a0500);
    border: 3px solid #ff4500;
    box-shadow: 
        inset 0 0 150px #8b0000,
        0 0 30px rgba(255, 69, 0, 0.4);
}

/* Give the buttons a "molten" look */
button {
    background: linear-gradient(180deg, #ff8c00, #ff4500);
    color: #1a0500;
    font-weight: bold;
    border-radius: 10px;
    border: 1px solid #ff0000;
    box-shadow: 0 4px 15px rgba(255, 0, 0, 0.5);
    transition: all 200ms ease-in-out;
}

button:hover {
    background: #ff0000;
    color: #ffffff;
    box-shadow: 0 0 20px #ff4500;
    transform: translateY(-2px);
}

.tableau-selected-card {
    box-shadow: 
        0 0 25px #ff8c00,
        0 0 10px #ffffff;
    border: 2px solid #ffffff;
}

.card-slot {
    border: 2px solid #4d1100;
    background-color: rgba(255, 69, 0, 0.05);
}

/* Testing texture/opacity churn */
.slot-emoji { 
    filter: drop-shadow(0 0 10px #ff4500);
    opacity: 0.6;
}

label.status-line {
    color: #ff8c00;
    text-shadow: 0 0 8px rgba(255, 140, 0, 0.6);
}"#;

const USERSTYLE_TEMPLATE_SAPPHIRE: &str = r#"/* Sapphire */
window,
window background,
box,
label {
  color: #eef4ff;
}

.board-background {
  background-image:
    radial-gradient(circle at 18% 15%, rgba(98, 166, 255, 0.16), transparent 42%),
    radial-gradient(circle at 85% 82%, rgba(53, 112, 255, 0.14), transparent 42%),
    linear-gradient(160deg, #0d1b4a, #10245d 48%, #0a173c 100%);
  border: 2px solid rgba(146, 187, 255, 0.42);
  box-shadow: 0 14px 36px rgba(0, 0, 0, 0.48);
}

button {
  color: #f7fbff;
  border: 1px solid rgba(170, 205, 255, 0.46);
  background-image: linear-gradient(180deg, #355dc2, #233f8a);
}

.status-line {
  color: #d6e7ff;
}
"#;

const USERSTYLE_TEMPLATE_AQUAMARINE: &str = r#"/* Aquamarine */
window,
window background,
box,
label {
  color: #eafff9;
}

.board-background {
  background-image:
    radial-gradient(circle at 14% 12%, rgba(138, 255, 229, 0.16), transparent 42%),
    radial-gradient(circle at 84% 82%, rgba(77, 230, 210, 0.15), transparent 44%),
    linear-gradient(160deg, #0e3a3d, #14555a 50%, #103337 100%);
  border: 2px solid rgba(170, 255, 236, 0.38);
  box-shadow: 0 14px 36px rgba(0, 0, 0, 0.44);
}

button {
  color: #f5fffd;
  border: 1px solid rgba(180, 255, 240, 0.42);
  background-image: linear-gradient(180deg, #2a8f97, #1f6f76);
}

.status-line {
  color: #c7fff4;
}
"#;

const USERSTYLE_TEMPLATE_CITRINE: &str = r#"/* Citrine */
window,
window background,
box,
label {
  color: #fff8e7;
}

.board-background {
  background-image:
    radial-gradient(circle at 16% 14%, rgba(255, 223, 126, 0.18), transparent 42%),
    radial-gradient(circle at 84% 82%, rgba(255, 194, 72, 0.15), transparent 44%),
    linear-gradient(160deg, #4a2a08, #5c350f 48%, #3a2107 100%);
  border: 2px solid rgba(255, 219, 142, 0.38);
  box-shadow: 0 14px 36px rgba(0, 0, 0, 0.45);
}

button {
  color: #fff9ec;
  border: 1px solid rgba(255, 232, 170, 0.44);
  background-image: linear-gradient(180deg, #c08c2b, #9a6d1f);
}

.status-line {
  color: #ffe6a8;
}
"#;

const USERSTYLE_TEMPLATE_MOONSTONE: &str = r#"/* Moonstone */
window,
window background,
box,
label {
  color: #f3f7ff;
}

.board-background {
  background-image:
    radial-gradient(circle at 17% 13%, rgba(214, 230, 255, 0.16), transparent 42%),
    radial-gradient(circle at 83% 83%, rgba(170, 194, 255, 0.14), transparent 44%),
    linear-gradient(160deg, #1e2537, #2a3046 52%, #1a2233 100%);
  border: 2px solid rgba(206, 220, 255, 0.34);
  box-shadow: 0 14px 34px rgba(0, 0, 0, 0.46);
}

button {
  color: #f7faff;
  border: 1px solid rgba(210, 226, 255, 0.40);
  background-image: linear-gradient(180deg, #5b6e97, #455576);
}

.status-line {
  color: #deebff;
}
"#;

const USERSTYLE_TEMPLATE_GARNET: &str = r#"/* Garnet */
window, window background, box, label { color: #ffeef2; }
.board-background {
  background-image:
    radial-gradient(circle at 16% 14%, rgba(255, 112, 145, 0.16), transparent 42%),
    radial-gradient(circle at 84% 82%, rgba(176, 33, 69, 0.16), transparent 44%),
    linear-gradient(160deg, #2c0712, #3e0a1b 52%, #22060f 100%);
  border: 2px solid rgba(228, 102, 133, 0.42);
}
button { color: #fff4f7; background-image: linear-gradient(180deg, #9d2948, #7b1f39); border: 1px solid rgba(255, 165, 187, 0.40); }
.status-line { color: #ffd0db; }
"#;

const USERSTYLE_TEMPLATE_AMETHYST: &str = r#"/* Amethyst */
window, window background, box, label { color: #f8f1ff; }
.board-background {
  background-image:
    radial-gradient(circle at 15% 12%, rgba(196, 145, 255, 0.16), transparent 40%),
    radial-gradient(circle at 86% 84%, rgba(134, 92, 222, 0.15), transparent 42%),
    linear-gradient(160deg, #26123f, #34205a 52%, #1d1032 100%);
  border: 2px solid rgba(189, 154, 255, 0.38);
}
button { color: #fbf7ff; background-image: linear-gradient(180deg, #7752bb, #5f4197); border: 1px solid rgba(208, 182, 255, 0.42); }
.status-line { color: #e9d9ff; }
"#;

const USERSTYLE_TEMPLATE_DIAMOND: &str = r#"/* Diamond */
window, window background, box, label { color: #f5fbff; }
.board-background {
  background-image:
    radial-gradient(circle at 14% 12%, rgba(220, 242, 255, 0.20), transparent 40%),
    radial-gradient(circle at 84% 82%, rgba(170, 210, 255, 0.15), transparent 42%),
    linear-gradient(160deg, #273847, #304a60 52%, #20303d 100%);
  border: 2px solid rgba(198, 229, 255, 0.42);
}
button { color: #f7fcff; background-image: linear-gradient(180deg, #68839f, #516b85); border: 1px solid rgba(207, 232, 255, 0.40); }
.status-line { color: #d8edff; }
"#;

const USERSTYLE_TEMPLATE_EMERALD: &str = r#"/* Emerald */
window, window background, box, label { color: #effff4; }
.board-background {
  background-image:
    radial-gradient(circle at 16% 12%, rgba(131, 255, 171, 0.16), transparent 40%),
    radial-gradient(circle at 84% 84%, rgba(48, 180, 103, 0.15), transparent 42%),
    linear-gradient(160deg, #0f3624, #165138 52%, #0d2c1f 100%);
  border: 2px solid rgba(149, 242, 184, 0.36);
}
button { color: #f4fff7; background-image: linear-gradient(180deg, #2f9362, #22734d); border: 1px solid rgba(173, 255, 204, 0.38); }
.status-line { color: #ccf7dc; }
"#;

const USERSTYLE_TEMPLATE_PEARL: &str = r#"/* Pearl */
window, window background, box, label { color: #fff8f0; }
.board-background {
  background-image:
    radial-gradient(circle at 14% 12%, rgba(255, 244, 229, 0.18), transparent 40%),
    radial-gradient(circle at 84% 84%, rgba(245, 226, 205, 0.16), transparent 44%),
    linear-gradient(160deg, #6c5e55, #7c6f65 52%, #584d45 100%);
  border: 2px solid rgba(248, 231, 213, 0.42);
}
button { color: #fffdf9; background-image: linear-gradient(180deg, #a48d78, #886f5d); border: 1px solid rgba(255, 237, 219, 0.40); }
.status-line { color: #ffe9d4; }
"#;

const USERSTYLE_TEMPLATE_RUBY: &str = r#"/* Ruby */
window, window background, box, label { color: #fff0f5; }
.board-background {
  background-image:
    radial-gradient(circle at 16% 14%, rgba(255, 112, 160, 0.16), transparent 42%),
    radial-gradient(circle at 84% 82%, rgba(209, 22, 74, 0.18), transparent 44%),
    linear-gradient(160deg, #300712, #490a1d 52%, #25060f 100%);
  border: 2px solid rgba(241, 88, 138, 0.42);
}
button { color: #fff5f8; background-image: linear-gradient(180deg, #bf2756, #971f44); border: 1px solid rgba(255, 166, 196, 0.42); }
.status-line { color: #ffc7db; }
"#;

const USERSTYLE_TEMPLATE_PERIDOT: &str = r#"/* Peridot */
window, window background, box, label { color: #f8ffe8; }
.board-background {
  background-image:
    radial-gradient(circle at 15% 12%, rgba(210, 255, 121, 0.16), transparent 40%),
    radial-gradient(circle at 84% 82%, rgba(140, 193, 44, 0.15), transparent 42%),
    linear-gradient(160deg, #2d3f12, #3c5519 52%, #25350f 100%);
  border: 2px solid rgba(200, 236, 120, 0.38);
}
button { color: #fbffef; background-image: linear-gradient(180deg, #8cad35, #728e2b); border: 1px solid rgba(226, 250, 169, 0.42); }
.status-line { color: #e0f6b0; }
"#;

const USERSTYLE_TEMPLATE_OPAL: &str = r#"/* Opal */
window, window background, box, label { color: #fff7ff; }
.board-background {
  background-image:
    radial-gradient(circle at 12% 10%, rgba(255, 179, 228, 0.14), transparent 38%),
    radial-gradient(circle at 86% 86%, rgba(161, 255, 255, 0.14), transparent 40%),
    radial-gradient(circle at 50% 50%, rgba(197, 206, 255, 0.10), transparent 56%),
    linear-gradient(160deg, #3b3347, #4a405b 52%, #2f283a 100%);
  border: 2px solid rgba(226, 211, 255, 0.34);
}
button { color: #fffaff; background-image: linear-gradient(180deg, #8d79a8, #73638b); border: 1px solid rgba(233, 221, 255, 0.38); }
.status-line { color: #eddfff; }
"#;

const USERSTYLE_TEMPLATE_TOPAZ: &str = r#"/* Topaz */
window, window background, box, label { color: #fff8ec; }
.board-background {
  background-image:
    radial-gradient(circle at 14% 12%, rgba(255, 196, 112, 0.16), transparent 40%),
    radial-gradient(circle at 84% 82%, rgba(231, 135, 38, 0.16), transparent 42%),
    linear-gradient(160deg, #4c2a0a, #5f3810 52%, #3d2208 100%);
  border: 2px solid rgba(249, 182, 109, 0.40);
}
button { color: #fff9ef; background-image: linear-gradient(180deg, #c57f2a, #a9681f); border: 1px solid rgba(255, 207, 143, 0.40); }
.status-line { color: #ffd9a4; }
"#;

const USERSTYLE_TEMPLATE_TURQUOISE: &str = r#"/* Turquoise */
window, window background, box, label { color: #edfffe; }
.board-background {
  background-image:
    radial-gradient(circle at 16% 12%, rgba(128, 255, 244, 0.16), transparent 40%),
    radial-gradient(circle at 84% 84%, rgba(54, 196, 196, 0.15), transparent 42%),
    linear-gradient(160deg, #0c3d43, #14535a 52%, #0a3136 100%);
  border: 2px solid rgba(144, 236, 230, 0.38);
}
button { color: #f4ffff; background-image: linear-gradient(180deg, #2f9ea3, #237f83); border: 1px solid rgba(178, 255, 247, 0.40); }
.status-line { color: #c5f6f1; }
"#;

impl CardthropicWindow {
    pub(super) fn userstyle_preset_names() -> &'static [&'static str] {
        &USERSTYLE_PRESET_NAMES
    }

    pub(super) fn default_userstyle_css() -> &'static str {
        USERSTYLE_TEMPLATE_NEON
    }

    pub(super) fn userstyle_css_for_preset(index: u32) -> Option<&'static str> {
        match index {
            1 => Some(USERSTYLE_TEMPLATE_SYSTEM),
            2 => Some(USERSTYLE_TEMPLATE_LIGHT_MODE),
            3 => Some(USERSTYLE_TEMPLATE_TERMINAL),
            4 => Some(USERSTYLE_TEMPLATE_NEON),
            5 => Some(USERSTYLE_TEMPLATE_GARNET),
            6 => Some(USERSTYLE_TEMPLATE_AMETHYST),
            7 => Some(USERSTYLE_TEMPLATE_AQUAMARINE),
            8 => Some(USERSTYLE_TEMPLATE_DIAMOND),
            9 => Some(USERSTYLE_TEMPLATE_EMERALD),
            10 => Some(USERSTYLE_TEMPLATE_PEARL),
            11 => Some(USERSTYLE_TEMPLATE_RUBY),
            12 => Some(USERSTYLE_TEMPLATE_PERIDOT),
            13 => Some(USERSTYLE_TEMPLATE_SAPPHIRE),
            14 => Some(USERSTYLE_TEMPLATE_OPAL),
            15 => Some(USERSTYLE_TEMPLATE_TOPAZ),
            16 => Some(USERSTYLE_TEMPLATE_TURQUOISE),
            17 => Some(USERSTYLE_TEMPLATE_MOONSTONE),
            18 => Some(USERSTYLE_TEMPLATE_CITRINE),
            19 => Some(USERSTYLE_TEMPLATE_CRT),
            20 => Some(USERSTYLE_TEMPLATE_MAGMA),
            _ => None,
        }
    }

    pub(super) fn userstyle_preset_for_css(css: &str) -> u32 {
        if css == USERSTYLE_TEMPLATE_SYSTEM {
            1
        } else if css == USERSTYLE_TEMPLATE_LIGHT_MODE {
            2
        } else if css == USERSTYLE_TEMPLATE_TERMINAL {
            3
        } else if css == USERSTYLE_TEMPLATE_NEON
            || css == USERSTYLE_TEMPLATE_CARDTHROPIC
            || css == USERSTYLE_TEMPLATE_CARDTHROPIC_MIDNIGHT
            || css == USERSTYLE_TEMPLATE_ARCADE
            || css == USERSTYLE_TEMPLATE_NOIR
        {
            4
        } else if css == USERSTYLE_TEMPLATE_GARNET {
            5
        } else if css == USERSTYLE_TEMPLATE_AMETHYST {
            6
        } else if css == USERSTYLE_TEMPLATE_AQUAMARINE || css == USERSTYLE_TEMPLATE_FOREST {
            7
        } else if css == USERSTYLE_TEMPLATE_DIAMOND {
            8
        } else if css == USERSTYLE_TEMPLATE_EMERALD {
            9
        } else if css == USERSTYLE_TEMPLATE_PEARL {
            10
        } else if css == USERSTYLE_TEMPLATE_RUBY {
            11
        } else if css == USERSTYLE_TEMPLATE_PERIDOT {
            12
        } else if css == USERSTYLE_TEMPLATE_SAPPHIRE {
            13
        } else if css == USERSTYLE_TEMPLATE_OPAL {
            14
        } else if css == USERSTYLE_TEMPLATE_TOPAZ {
            15
        } else if css == USERSTYLE_TEMPLATE_TURQUOISE {
            16
        } else if css == USERSTYLE_TEMPLATE_MOONSTONE {
            17
        } else if css == USERSTYLE_TEMPLATE_CITRINE {
            18
        } else if css == USERSTYLE_TEMPLATE_CRT {
            19
        } else if css == USERSTYLE_TEMPLATE_MAGMA {
            20
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
            .default_height(645)
            .build();
        let committed_css = Rc::new(RefCell::new(
            self.imp().custom_userstyle_css.borrow().clone(),
        ));
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
            "Editing Custom CSS. Import a preset to replace Custom, preview fast, then save.",
        ));
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.add_css_class("dim-label");
        root.append(&subtitle);

        let presets_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let presets_label = gtk::Label::new(Some("Import from Preset"));
        presets_label.set_xalign(0.0);
        presets_label.add_css_class("dim-label");
        let import_preset_names: Vec<String> = Self::userstyle_preset_names()
            .iter()
            .skip(1)
            .map(|name| (*name).to_string())
            .collect();
        let import_preset_name_refs: Vec<&str> = import_preset_names
            .iter()
            .map(|name| name.as_str())
            .collect();
        let presets_dropdown = gtk::DropDown::from_strings(&import_preset_name_refs);
        // Keep import unset initially so the field is visually blank until user chooses a preset.
        presets_dropdown.set_selected(gtk::INVALID_LIST_POSITION);
        presets_dropdown.set_hexpand(true);

        let font_sizes: [u32; 10] = [8, 9, 10, 11, 12, 13, 14, 16, 18, 20];
        let font_size_label = gtk::Label::new(Some("IDE Font Size"));
        font_size_label.set_xalign(0.0);
        font_size_label.add_css_class("dim-label");
        let font_size_dropdown = gtk::DropDown::from_strings(&[
            "8", "9", "10", "11", "12", "13", "14", "16", "18", "20",
        ]);
        font_size_dropdown.set_selected(3);

        presets_row.append(&presets_label);
        presets_row.append(&presets_dropdown);
        presets_row.append(&font_size_label);
        presets_row.append(&font_size_dropdown);
        root.append(&presets_row);

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
        editor_font_provider.load_from_string(".userstyle-editor-view { font-size: 11pt; }");

        let hint = gtk::Label::new(Some(
            "Tip: target classes like .board-background, .card-slot, .keyboard-focus-card.",
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

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let save = gtk::Button::with_label("Save Changes");
        save.add_css_class("suggested-action");
        let close = gtk::Button::with_label("Discard Changes");
        close.add_css_class("flat");
        actions.append(&save);
        actions.append(&close);
        root.append(&actions);

        font_size_dropdown.connect_selected_notify(glib::clone!(
            #[strong]
            editor_font_provider,
            move |dropdown| {
                let selected = dropdown.selected() as usize;
                let size = *font_sizes.get(selected).unwrap_or(&11);
                editor_font_provider.load_from_string(&format!(
                    ".userstyle-editor-view {{ font-size: {}pt; }}",
                    size
                ));
            }
        ));

        presets_dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            status,
            #[weak]
            live_preview,
            #[strong]
            import_preset_names,
            move |dropdown| {
                let import_idx = dropdown.selected() as usize;
                let preset_idx = dropdown.selected() + 1;
                if let Some(css) = Self::userstyle_css_for_preset(preset_idx) {
                    buffer.set_text(css);
                    if live_preview.is_active() {
                        // Import previews immediately only when live preview is enabled.
                        window.apply_custom_userstyle(css, false);
                    }
                    if let Some(name) = import_preset_names.get(import_idx) {
                        status.set_label(&format!("Imported {name} into Custom CSS"));
                    }
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

        save.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            status,
            #[weak]
            diagnostics,
            #[strong]
            committed_css,
            #[weak]
            dialog,
            move |_| {
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                window.apply_custom_userstyle(&text, true);
                *committed_css.borrow_mut() = text.clone();
                status.set_label("Changes saved");
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
                dialog.close();
            }
        ));

        close.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            committed_css,
            #[weak]
            dialog,
            move |_| {
                let css = committed_css.borrow().clone();
                window.apply_custom_userstyle(&css, false);
                dialog.close();
            }
        ));

        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            committed_css,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                let css = committed_css.borrow().clone();
                window.apply_custom_userstyle(&css, false);
                *window.imp().custom_userstyle_dialog.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));

        dialog.set_child(Some(&root));
        *self.imp().custom_userstyle_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }
}
