use super::*;
use sourceview5::prelude::*;

const USERSTYLE_PRESET_NAMES: [&str; 19] = [
    "Custom",
    "System",
    "Cardthropic",
    "CRT",
    "Magma",
    "Garnet (January, Deep Red)",
    "Amethyst (February, Purple)",
    "Aquamarine (March, Blue-Green)",
    "Diamond (April, Clear White)",
    "Emerald (May, Green)",
    "Pearl (June, White)",
    "Ruby (July, Red)",
    "Peridot (August, Olive Green)",
    "Sapphire (September, Blue)",
    "Opal (October, Iridescent White)",
    "Topaz (November, Golden Yellow)",
    "Turquoise (December, Cyan Blue)",
    "Moonstone (June, Milky White)",
    "Citrine (November, Amber Yellow)",
];

const USERSTYLE_TEMPLATE_SYSTEM: &str = r#"/* System
Minimal override: let GNOME theme drive appearance.
*/
.board-background {
  background-image: none;
  background-color: @window_bg_color;
}
"#;

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

macro_rules! deep_theme_template {
    (
        $name:literal,
        $text:literal,
        $header_bg:literal,
        $board_a:literal,
        $board_b:literal,
        $board_c:literal,
        $glow_a:literal,
        $glow_b:literal,
        $border:literal,
        $button_a:literal,
        $button_b:literal,
        $input_bg:literal,
        $input_border:literal,
        $accent:literal,
        $dim:literal
    ) => {
        concat!(
            "/* ", $name, "\n",
            "Cardthropic Theme Reference Template\n",
            "You can copy this into Custom CSS and tweak values section-by-section.\n",
            "Target map:\n",
            "  1) Global text/surfaces: window, box, label, headerbar, popover, frame\n",
            "  2) Board surface: .board-background\n",
            "  3) Inputs/buttons: button, entry, combobox, dropdown\n",
            "  4) Status/HUD text: .status-line, .stats-line, .dim-label\n",
            "  5) Card interaction states:\n",
            "     .tableau-selected-card, .waste-selected-card, .smart-move-fail-flash,\n",
            "     .keyboard-focus-card, .keyboard-focus-empty\n",
            "  6) Empty-slot chrome: .card-slot, .card-slot:drop(active), .tableau-drop-target:drop(active)\n",
            "  7) Motion accents: .hint-invert, .motion-fly-card\n",
            "  8) Seed check fields: entry.seed-winnable / entry.seed-unwinnable\n",
            "Note: actual face card artwork is SVG texture-based and not recolored by CSS.\n",
            "*/\n",
            "window,\nwindow background,\nbox,\nlabel {\n  color: ", $text, ";\n}\n\n",
            "headerbar,\npopover,\nmenu,\nframe {\n  background: ", $header_bg, ";\n}\n\n",
            ".board-background {\n",
            "  background-image:\n",
            "    radial-gradient(circle at 14% 12%, ", $glow_a, ", transparent 40%),\n",
            "    radial-gradient(circle at 84% 84%, ", $glow_b, ", transparent 42%),\n",
            "    linear-gradient(160deg, ", $board_a, ", ", $board_b, " 52%, ", $board_c, " 100%);\n",
            "  border: 2px solid ", $border, ";\n",
            "  box-shadow:\n",
            "    0 0 0 1px alpha(", $border, ", 0.22),\n",
            "    0 12px 34px alpha(#000000, 0.46);\n",
            "}\n\n",
            "button {\n",
            "  color: ", $text, ";\n",
            "  border-radius: 11px;\n",
            "  border: 1px solid alpha(", $border, ", 0.85);\n",
            "  background-image: linear-gradient(180deg, ", $button_a, ", ", $button_b, ");\n",
            "}\n\n",
            "button:hover {\n",
            "  border-color: alpha(", $accent, ", 0.90);\n",
            "}\n\n",
            "button:active {\n",
            "  background-image: linear-gradient(180deg, ", $button_b, ", ", $button_a, ");\n",
            "}\n\n",
            ".hud-toggle:checked {\n",
            "  color: ", $accent, ";\n",
            "  border-color: alpha(", $accent, ", 0.75);\n",
            "  box-shadow: inset 0 0 0 1px alpha(", $accent, ", 0.30);\n",
            "  background-color: alpha(", $accent, ", 0.14);\n",
            "}\n\n",
            "entry,\ncombobox,\ndropdown,\npopover entry {\n",
            "  color: ", $text, ";\n",
            "  background: ", $input_bg, ";\n",
            "  border: 1px solid ", $input_border, ";\n",
            "}\n\n",
            ".status-line {\n  color: ", $text, ";\n  font-weight: 700;\n}\n\n",
            ".stats-line,\n.dim-label {\n  color: ", $dim, ";\n}\n\n",
            ".tableau-selected-card,\n.waste-selected-card {\n",
            "  box-shadow:\n",
            "    inset 0 0 0 2px alpha(", $accent, ", 0.95),\n",
            "    0 0 10px alpha(", $accent, ", 0.34);\n",
            "  background-color: alpha(", $accent, ", 0.12);\n",
            "}\n\n",
            ".smart-move-fail-flash {\n",
            "  box-shadow:\n",
            "    inset 0 0 0 3px alpha(", $accent, ", 1.0),\n",
            "    0 0 14px alpha(", $accent, ", 0.55);\n",
            "  background-color: alpha(", $accent, ", 0.24);\n",
            "}\n\n",
            ".keyboard-focus-card {\n",
            "  box-shadow:\n",
            "    inset 0 0 0 2px #0d1020,\n",
            "    inset 0 0 0 5px #ffffff,\n",
            "    inset 0 0 0 8px ", $accent, ";\n",
            "}\n\n",
            ".keyboard-focus-empty {\n",
            "  box-shadow: inset 0 0 0 3px alpha(", $accent, ", 0.92);\n",
            "  background-color: alpha(", $accent, ", 0.12);\n",
            "}\n\n",
            ".card-slot {\n",
            "  border: 1px solid alpha(", $border, ", 0.62);\n",
            "  background-color: alpha(", $border, ", 0.08);\n",
            "}\n\n",
            ".card-slot:drop(active),\n.tableau-drop-target:drop(active) {\n",
            "  border: 1px solid alpha(", $accent, ", 0.62);\n",
            "  background-color: alpha(", $accent, ", 0.10);\n",
            "  box-shadow: none;\n",
            "  outline: none;\n",
            "}\n\n",
            ".slot-emoji {\n  opacity: 0.78;\n}\n\n",
            ".hint-invert {\n",
            "  filter: invert(1);\n",
            "  color: ", $accent, ";\n",
            "  background-color: alpha(", $accent, ", 0.12);\n",
            "}\n\n",
            ".motion-fly-card {\n",
            "  box-shadow:\n",
            "    0 2px 10px alpha(#000000, 0.44),\n",
            "    0 0 0 1px alpha(", $accent, ", 0.30);\n",
            "}\n\n",
            "entry.seed-winnable {\n",
            "  background-color: alpha(#2ecc71, 0.22);\n",
            "  border-color: alpha(#2ecc71, 0.85);\n",
            "}\n\n",
            "entry.seed-unwinnable {\n",
            "  background-color: alpha(#e74c3c, 0.22);\n",
            "  border-color: alpha(#e74c3c, 0.85);\n",
            "}\n"
        )
    };
}

const THEME_PRESET_CARDTHROPIC: &str = r#"/* Cardthropic
Signature neon/glass look.
Target map: window/headerbar/popover, .board-background, button/input, status text,
selection/focus states, .card-slot drop states, motion, seed result entries.
*/
window, window background, box, label { color: #fff4ff; }
headerbar, popover, menu, frame { background: rgba(18, 10, 36, 0.58); }

.board-background {
  background-image:
    radial-gradient(circle at 12% 10%, rgba(255, 0, 140, 0.34), transparent 40%),
    radial-gradient(circle at 88% 88%, rgba(0, 236, 255, 0.30), transparent 42%),
    linear-gradient(145deg, #3b1868, #1f3f82 50%, #0a3363 100%);
  border: 2px solid rgba(255, 126, 224, 0.56);
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.24),
    0 0 26px rgba(255, 0, 170, 0.34),
    0 0 40px rgba(0, 214, 255, 0.24);
}

button {
  color: #fff8ff;
  border-radius: 12px;
  border: 1px solid rgba(255, 147, 233, 0.78);
  background-image: linear-gradient(180deg, #8442a2, #563274);
}
button:hover { border-color: rgba(247, 153, 255, 0.95); }
button:active { background-image: linear-gradient(180deg, #563274, #8442a2); }

.hud-toggle:checked {
  color: #ff9bff;
  border-color: rgba(255, 155, 255, 0.72);
  background-color: rgba(255, 90, 235, 0.14);
  box-shadow: inset 0 0 0 1px rgba(255, 155, 255, 0.26);
}

entry, combobox, dropdown, popover entry {
  color: #fff6ff;
  background: rgba(14, 18, 34, 0.62);
  border: 1px solid rgba(211, 179, 255, 0.38);
}

.status-line { color: #ffe9fc; font-weight: 700; }
.stats-line, .dim-label { color: rgba(247, 221, 255, 0.86); }

.tableau-selected-card, .waste-selected-card {
  box-shadow: inset 0 0 0 2px rgba(246, 120, 255, 0.96), 0 0 12px rgba(111, 231, 255, 0.32);
  background-color: rgba(233, 124, 255, 0.12);
}
.smart-move-fail-flash {
  box-shadow: inset 0 0 0 3px rgba(246, 120, 255, 1.0), 0 0 16px rgba(246, 120, 255, 0.54);
  background-color: rgba(246, 120, 255, 0.24);
}
.keyboard-focus-card {
  box-shadow: inset 0 0 0 2px #0d1020, inset 0 0 0 5px #ffffff, inset 0 0 0 8px #f678ff;
}
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(111, 231, 255, 0.92);
  background-color: rgba(111, 231, 255, 0.12);
}

.card-slot { border: 1px solid rgba(213, 176, 255, 0.60); background-color: rgba(213, 176, 255, 0.08); }
.card-slot:drop(active), .tableau-drop-target:drop(active) {
  border: 1px solid rgba(111, 231, 255, 0.62);
  background-color: rgba(111, 231, 255, 0.10);
  box-shadow: none;
  outline: none;
}
.slot-emoji { opacity: 0.86; }

.hint-invert { filter: invert(1); color: #6fe7ff; background-color: rgba(111, 231, 255, 0.10); }
.motion-fly-card { box-shadow: 0 2px 10px rgba(0, 0, 0, 0.44), 0 0 0 1px rgba(246, 120, 255, 0.32); }

entry.seed-winnable { background-color: rgba(46, 204, 113, 0.22); border-color: rgba(46, 204, 113, 0.85); }
entry.seed-unwinnable { background-color: rgba(231, 76, 60, 0.22); border-color: rgba(231, 76, 60, 0.85); }
"#;

const THEME_PRESET_CRT: &str = r#"/* CRT
Retro phosphor monitor style.
Target map:
  1) Global surfaces/text: window, headerbar, popover, frame, label
  2) Board skin: .board-background
  3) Controls: button, entry, dropdown
  4) Card interaction states: selected/focus/empty-focus
  5) Slot styling: .card-slot and drop targets
  6) Extras: status text, hint/motion classes, seed result entries
*/
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
button:hover {
  border-color: rgba(99, 255, 169, 0.80);
}
button:active {
  background-image: linear-gradient(180deg, #052313, #08301b);
}

.hud-toggle:checked {
  color: #8dffbd;
  border-color: rgba(99, 255, 169, 0.65);
  background-color: rgba(16, 255, 134, 0.12);
  box-shadow: inset 0 0 0 1px rgba(99, 255, 169, 0.30);
}

entry, combobox, dropdown, popover entry {
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
    0 0 14px rgba(16, 255, 134, 0.55),
    inset 0 0 0 2px rgba(16, 255, 134, 0.88);
  background-color: rgba(16, 255, 134, 0.10);
}
.smart-move-fail-flash {
  box-shadow:
    inset 0 0 0 3px rgba(16, 255, 134, 1.0),
    0 0 16px rgba(16, 255, 134, 0.62);
  background-color: rgba(16, 255, 134, 0.20);
}
.keyboard-focus-card {
  box-shadow:
    inset 0 0 0 2px #031008,
    inset 0 0 0 5px #e6ffe1,
    inset 0 0 0 8px rgba(16, 255, 134, 0.95);
}
.keyboard-focus-empty {
  box-shadow: inset 0 0 0 3px rgba(16, 255, 134, 0.90);
  background-color: rgba(16, 255, 134, 0.12);
}

.card-slot {
  border: 1px solid rgba(90, 255, 164, 0.30);
  background-color: rgba(16, 255, 134, 0.04);
}

.card-slot:drop(active),
.tableau-drop-target:drop(active) {
  border: 1px solid rgba(99, 255, 169, 0.52);
  background-color: rgba(16, 255, 134, 0.09);
  box-shadow: none;
  outline: none;
}
.slot-emoji {
  opacity: 0.84;
}

.hint-invert {
  filter: invert(1);
  color: #16ff86;
  background-color: rgba(16, 255, 134, 0.10);
}

.motion-fly-card {
  box-shadow:
    0 2px 10px rgba(0, 0, 0, 0.46),
    0 0 0 1px rgba(16, 255, 134, 0.34);
}

entry.seed-winnable { background-color: rgba(46, 204, 113, 0.22); border-color: rgba(46, 204, 113, 0.85); }
entry.seed-unwinnable { background-color: rgba(231, 76, 60, 0.22); border-color: rgba(231, 76, 60, 0.85); }
"#;

const THEME_PRESET_MAGMA: &str = r#"/* Magma Core
Lava-intense signature preset.
Target map:
  1) Global surfaces/text: window, headerbar, popover, frame, label
  2) Board skin: .board-background
  3) Controls: button, entry, dropdown
  4) Card interaction states: .tableau-selected-card / .waste-selected-card / focus classes
  5) Slot styling: .card-slot and drop targets
  6) Extras: status text, hint/motion classes, seed result entries
*/
window, window background { background: #1a0500; }
box, label { color: #ffd9bd; }
headerbar, popover, menu, frame { background: rgba(30, 8, 2, 0.82); }

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

button:active {
    background: linear-gradient(180deg, #ff4500, #ff8c00);
}

.hud-toggle:checked {
    color: #ffd08a;
    border-color: #ff8c00;
    box-shadow: inset 0 0 0 1px rgba(255, 140, 0, 0.35), 0 0 14px rgba(255, 69, 0, 0.35);
    background-color: rgba(255, 69, 0, 0.20);
}

entry, combobox, dropdown, popover entry {
    color: #ffe7d2;
    background: rgba(35, 8, 2, 0.86);
    border: 1px solid rgba(255, 90, 30, 0.58);
}

.tableau-selected-card,
.waste-selected-card {
    box-shadow:
        0 0 25px #ff8c00,
        0 0 10px #ffffff;
    border: 2px solid #ffffff;
}

.smart-move-fail-flash {
    box-shadow:
        inset 0 0 0 3px #ff4500,
        0 0 18px rgba(255, 69, 0, 0.75);
    background-color: rgba(255, 69, 0, 0.28);
}

.keyboard-focus-card {
    box-shadow:
        inset 0 0 0 2px #2b0a00,
        inset 0 0 0 5px #fff5eb,
        inset 0 0 0 8px #ff8c00;
}

.keyboard-focus-empty {
    box-shadow: inset 0 0 0 3px #ff8c00;
    background-color: rgba(255, 69, 0, 0.16);
}

.card-slot {
    border: 2px solid #4d1100;
    background-color: rgba(255, 69, 0, 0.05);
}

.card-slot:drop(active),
.tableau-drop-target:drop(active) {
    border: 2px solid #ff8c00;
    background-color: rgba(255, 140, 0, 0.12);
    box-shadow: none;
    outline: none;
}

/* Testing texture/opacity churn */
.slot-emoji {
    filter: drop-shadow(0 0 10px #ff4500);
    opacity: 0.6;
}

label.status-line,
.status-line {
    color: #ff8c00;
    text-shadow: 0 0 8px rgba(255, 140, 0, 0.6);
}

.stats-line,
.dim-label {
    color: rgba(255, 186, 130, 0.90);
}

.hint-invert {
    filter: invert(1);
    color: #ffb463;
    background-color: rgba(255, 140, 0, 0.14);
}

.motion-fly-card {
    box-shadow:
        0 2px 10px rgba(0, 0, 0, 0.48),
        0 0 0 1px rgba(255, 69, 0, 0.42);
}

entry.seed-winnable {
    background-color: rgba(46, 204, 113, 0.22);
    border-color: rgba(46, 204, 113, 0.85);
}

entry.seed-unwinnable {
    background-color: rgba(231, 76, 60, 0.22);
    border-color: rgba(231, 76, 60, 0.85);
}
"#;

const THEME_PRESET_GARNET: &str = deep_theme_template!(
    "Garnet",
    "#fff1f3",
    "rgba(46, 9, 15, 0.68)",
    "#420c1a",
    "#5d1126",
    "#2a0811",
    "rgba(255, 82, 120, 0.16)",
    "rgba(196, 30, 58, 0.16)",
    "rgba(245, 124, 145, 0.40)",
    "#c13556",
    "#8f233f",
    "rgba(38, 10, 15, 0.84)",
    "rgba(255, 148, 170, 0.42)",
    "#e63b68",
    "rgba(255, 206, 216, 0.84)"
);

const THEME_PRESET_AMETHYST: &str = deep_theme_template!(
    "Amethyst",
    "#f8efff",
    "rgba(34, 20, 52, 0.66)",
    "#3d2361",
    "#523180",
    "#2a1842",
    "rgba(193, 116, 255, 0.16)",
    "rgba(138, 87, 255, 0.13)",
    "rgba(203, 158, 255, 0.40)",
    "#9b67de",
    "#7246ac",
    "rgba(30, 22, 47, 0.82)",
    "rgba(210, 179, 255, 0.40)",
    "#b67aff",
    "rgba(226, 205, 255, 0.84)"
);

const THEME_PRESET_AQUAMARINE: &str = deep_theme_template!(
    "Aquamarine",
    "#edfffb",
    "rgba(12, 47, 51, 0.66)",
    "#0f4d57",
    "#146976",
    "#0b343a",
    "rgba(102, 255, 235, 0.14)",
    "rgba(54, 190, 210, 0.15)",
    "rgba(140, 237, 226, 0.42)",
    "#2baea3",
    "#1c8b83",
    "rgba(10, 39, 42, 0.82)",
    "rgba(156, 246, 238, 0.42)",
    "#52f5da",
    "rgba(195, 255, 246, 0.84)"
);

const THEME_PRESET_DIAMOND: &str = deep_theme_template!(
    "Diamond",
    "#f7fbff",
    "rgba(24, 30, 44, 0.66)",
    "#283349",
    "#344462",
    "#1b2433",
    "rgba(182, 217, 255, 0.16)",
    "rgba(130, 180, 255, 0.12)",
    "rgba(201, 223, 255, 0.40)",
    "#6c8ec5",
    "#4f6d9b",
    "rgba(20, 28, 43, 0.84)",
    "rgba(205, 225, 255, 0.42)",
    "#8eb8ff",
    "rgba(215, 230, 255, 0.84)"
);

const THEME_PRESET_EMERALD: &str = deep_theme_template!(
    "Emerald",
    "#effff1",
    "rgba(16, 38, 22, 0.67)",
    "#1a4d2a",
    "#23693a",
    "#123520",
    "rgba(120, 255, 158, 0.15)",
    "rgba(50, 200, 112, 0.14)",
    "rgba(138, 234, 170, 0.40)",
    "#2eac5f",
    "#228447",
    "rgba(10, 33, 18, 0.84)",
    "rgba(151, 245, 184, 0.42)",
    "#4be07f",
    "rgba(194, 255, 212, 0.84)"
);

const THEME_PRESET_PEARL: &str = deep_theme_template!(
    "Pearl",
    "#fffaf2",
    "rgba(58, 50, 38, 0.60)",
    "#6f6654",
    "#8b816d",
    "#4a4235",
    "rgba(255, 238, 207, 0.16)",
    "rgba(214, 197, 168, 0.14)",
    "rgba(250, 226, 188, 0.38)",
    "#b89e78",
    "#8f7858",
    "rgba(50, 43, 33, 0.78)",
    "rgba(247, 224, 186, 0.42)",
    "#dec49d",
    "rgba(255, 233, 200, 0.84)"
);

const THEME_PRESET_RUBY: &str = deep_theme_template!(
    "Ruby",
    "#fff1f6",
    "rgba(47, 9, 24, 0.66)",
    "#4d1028",
    "#6f1738",
    "#310a1d",
    "rgba(255, 112, 160, 0.17)",
    "rgba(209, 22, 74, 0.16)",
    "rgba(245, 123, 159, 0.40)",
    "#c03a66",
    "#8f274a",
    "rgba(39, 9, 22, 0.84)",
    "rgba(255, 162, 198, 0.42)",
    "#f04f86",
    "rgba(255, 207, 226, 0.84)"
);

const THEME_PRESET_PERIDOT: &str = deep_theme_template!(
    "Peridot",
    "#f8ffe8",
    "rgba(37, 45, 19, 0.66)",
    "#3b4f19",
    "#4e6a22",
    "#283714",
    "rgba(210, 255, 121, 0.15)",
    "rgba(140, 193, 44, 0.14)",
    "rgba(202, 236, 121, 0.40)",
    "#8caf35",
    "#6c862a",
    "rgba(30, 38, 14, 0.84)",
    "rgba(226, 250, 169, 0.42)",
    "#b7e94a",
    "rgba(226, 248, 178, 0.84)"
);

const THEME_PRESET_SAPPHIRE: &str = deep_theme_template!(
    "Sapphire",
    "#eef4ff",
    "rgba(12, 24, 53, 0.68)",
    "#122e66",
    "#1a3e84",
    "#0d1e47",
    "rgba(123, 174, 255, 0.16)",
    "rgba(66, 123, 224, 0.15)",
    "rgba(147, 190, 255, 0.42)",
    "#356fcc",
    "#2753a0",
    "rgba(11, 22, 47, 0.84)",
    "rgba(167, 203, 255, 0.42)",
    "#67a6ff",
    "rgba(197, 220, 255, 0.84)"
);

const THEME_PRESET_OPAL: &str = deep_theme_template!(
    "Opal",
    "#fff7ff",
    "rgba(44, 39, 54, 0.66)",
    "#4a415b",
    "#615470",
    "#373045",
    "rgba(255, 179, 228, 0.14)",
    "rgba(161, 255, 255, 0.12)",
    "rgba(227, 211, 255, 0.38)",
    "#9d87bf",
    "#78679a",
    "rgba(38, 33, 47, 0.82)",
    "rgba(234, 221, 255, 0.40)",
    "#d2b7ff",
    "rgba(234, 221, 255, 0.84)"
);

const THEME_PRESET_TOPAZ: &str = deep_theme_template!(
    "Topaz",
    "#fff8ec",
    "rgba(66, 38, 14, 0.66)",
    "#5d3510",
    "#7b4a15",
    "#46280d",
    "rgba(255, 196, 112, 0.16)",
    "rgba(231, 135, 38, 0.14)",
    "rgba(250, 182, 109, 0.40)",
    "#c57f2a",
    "#9d6520",
    "rgba(57, 33, 12, 0.82)",
    "rgba(255, 207, 143, 0.42)",
    "#ffbd59",
    "rgba(255, 220, 166, 0.84)"
);

const THEME_PRESET_TURQUOISE: &str = deep_theme_template!(
    "Turquoise",
    "#edfffe",
    "rgba(10, 43, 47, 0.66)",
    "#0d444a",
    "#125b63",
    "#0a3237",
    "rgba(128, 255, 244, 0.15)",
    "rgba(54, 196, 196, 0.14)",
    "rgba(144, 236, 230, 0.40)",
    "#2f9ea3",
    "#237f83",
    "rgba(9, 34, 37, 0.82)",
    "rgba(178, 255, 247, 0.40)",
    "#6ef7ed",
    "rgba(200, 255, 250, 0.84)"
);

const THEME_PRESET_MOONSTONE: &str = deep_theme_template!(
    "Moonstone",
    "#f8fbff",
    "rgba(35, 41, 52, 0.64)",
    "#495163",
    "#5c667b",
    "#353c4b",
    "rgba(210, 228, 255, 0.14)",
    "rgba(173, 201, 255, 0.13)",
    "rgba(215, 229, 255, 0.40)",
    "#8393af",
    "#667792",
    "rgba(29, 35, 47, 0.82)",
    "rgba(218, 231, 255, 0.42)",
    "#b8cbff",
    "rgba(219, 231, 252, 0.84)"
);

const THEME_PRESET_CITRINE: &str = deep_theme_template!(
    "Citrine",
    "#fff9ea",
    "rgba(56, 43, 14, 0.64)",
    "#6b5319",
    "#876b21",
    "#473810",
    "rgba(255, 224, 116, 0.15)",
    "rgba(222, 176, 55, 0.14)",
    "rgba(247, 214, 122, 0.40)",
    "#c49a2f",
    "#9b7723",
    "rgba(47, 36, 11, 0.82)",
    "rgba(248, 223, 146, 0.42)",
    "#ffd36b",
    "rgba(248, 227, 159, 0.84)"
);

impl CardthropicWindow {
    pub(super) fn is_system_userstyle_active(&self) -> bool {
        Self::is_system_userstyle_css(&self.imp().custom_userstyle_css.borrow())
    }

    fn is_system_userstyle_css(css: &str) -> bool {
        css == USERSTYLE_TEMPLATE_SYSTEM
    }

    pub(super) fn userstyle_preset_names() -> &'static [&'static str] {
        &USERSTYLE_PRESET_NAMES
    }

    pub(super) fn default_userstyle_css() -> &'static str {
        THEME_PRESET_CARDTHROPIC
    }

    pub(super) fn userstyle_css_for_preset(index: u32) -> Option<&'static str> {
        match index {
            1 => Some(USERSTYLE_TEMPLATE_SYSTEM),
            2 => Some(THEME_PRESET_CARDTHROPIC),
            3 => Some(THEME_PRESET_CRT),
            4 => Some(THEME_PRESET_MAGMA),
            5 => Some(THEME_PRESET_GARNET),
            6 => Some(THEME_PRESET_AMETHYST),
            7 => Some(THEME_PRESET_AQUAMARINE),
            8 => Some(THEME_PRESET_DIAMOND),
            9 => Some(THEME_PRESET_EMERALD),
            10 => Some(THEME_PRESET_PEARL),
            11 => Some(THEME_PRESET_RUBY),
            12 => Some(THEME_PRESET_PERIDOT),
            13 => Some(THEME_PRESET_SAPPHIRE),
            14 => Some(THEME_PRESET_OPAL),
            15 => Some(THEME_PRESET_TOPAZ),
            16 => Some(THEME_PRESET_TURQUOISE),
            17 => Some(THEME_PRESET_MOONSTONE),
            18 => Some(THEME_PRESET_CITRINE),
            _ => None,
        }
    }

    pub(super) fn userstyle_preset_for_css(css: &str) -> u32 {
        if Self::is_system_userstyle_css(css) {
            1
        } else if css == THEME_PRESET_CARDTHROPIC
            || css == USERSTYLE_TEMPLATE_NEON
            || css == USERSTYLE_TEMPLATE_CARDTHROPIC
            || css == USERSTYLE_TEMPLATE_CARDTHROPIC_MIDNIGHT
            || css == USERSTYLE_TEMPLATE_ARCADE
            || css == USERSTYLE_TEMPLATE_NOIR
        {
            2
        } else if css == THEME_PRESET_CRT || css == USERSTYLE_TEMPLATE_CRT {
            3
        } else if css == THEME_PRESET_MAGMA || css == USERSTYLE_TEMPLATE_MAGMA {
            4
        } else if css == THEME_PRESET_GARNET || css == USERSTYLE_TEMPLATE_GARNET {
            5
        } else if css == THEME_PRESET_AMETHYST || css == USERSTYLE_TEMPLATE_AMETHYST {
            6
        } else if css == THEME_PRESET_AQUAMARINE
            || css == USERSTYLE_TEMPLATE_AQUAMARINE
            || css == USERSTYLE_TEMPLATE_FOREST
        {
            7
        } else if css == THEME_PRESET_DIAMOND || css == USERSTYLE_TEMPLATE_DIAMOND {
            8
        } else if css == THEME_PRESET_EMERALD || css == USERSTYLE_TEMPLATE_EMERALD {
            9
        } else if css == THEME_PRESET_PEARL || css == USERSTYLE_TEMPLATE_PEARL {
            10
        } else if css == THEME_PRESET_RUBY || css == USERSTYLE_TEMPLATE_RUBY {
            11
        } else if css == THEME_PRESET_PERIDOT || css == USERSTYLE_TEMPLATE_PERIDOT {
            12
        } else if css == THEME_PRESET_SAPPHIRE || css == USERSTYLE_TEMPLATE_SAPPHIRE {
            13
        } else if css == THEME_PRESET_OPAL || css == USERSTYLE_TEMPLATE_OPAL {
            14
        } else if css == THEME_PRESET_TOPAZ || css == USERSTYLE_TEMPLATE_TOPAZ {
            15
        } else if css == THEME_PRESET_TURQUOISE || css == USERSTYLE_TEMPLATE_TURQUOISE {
            16
        } else if css == THEME_PRESET_MOONSTONE || css == USERSTYLE_TEMPLATE_MOONSTONE {
            17
        } else if css == THEME_PRESET_CITRINE || css == USERSTYLE_TEMPLATE_CITRINE {
            18
        } else {
            0
        }
    }

    pub(super) fn migrate_legacy_userstyle_css(css: &str) -> Option<&'static str> {
        if css == USERSTYLE_TEMPLATE_LIGHT_MODE || css == USERSTYLE_TEMPLATE_TERMINAL {
            Some(USERSTYLE_TEMPLATE_SYSTEM)
        } else {
            None
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
        let is_preset_css = Self::userstyle_preset_for_css(css) > 0;
        let scheme = if Self::is_system_userstyle_css(css) {
            adw::ColorScheme::Default
        } else {
            adw::ColorScheme::ForceDark
        };
        adw::StyleManager::default().set_color_scheme(scheme);

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
        if !is_preset_css {
            *imp.saved_custom_userstyle_css.borrow_mut() = css.to_string();
        }

        if Self::is_system_userstyle_css(css) {
            self.clear_board_color_override();
        } else {
            let board_color = imp.board_color_hex.borrow().clone();
            self.set_board_color(&board_color, false);
        }

        if persist {
            let settings = imp.settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_CUSTOM_USERSTYLE_CSS, css);
                if !is_preset_css {
                    let _ = settings.set_string(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS, css);
                }
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
        let allow_close = Rc::new(Cell::new(false));
        let committed_css = Rc::new(RefCell::new(
            self.imp().saved_custom_userstyle_css.borrow().clone(),
        ));
        dialog.set_transient_for(Some(self));
        dialog.set_hide_on_close(false);
        dialog.set_destroy_with_parent(true);

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

        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = gtk::Label::new(Some("Custom CSS Userstyle"));
        title.set_xalign(0.0);
        title.set_hexpand(true);
        title.add_css_class("title-4");
        title_row.append(&title);
        let unsaved_badge = gtk::Label::new(Some("Unsaved changes"));
        unsaved_badge.add_css_class("accent");
        unsaved_badge.set_visible(false);
        title_row.append(&unsaved_badge);
        root.append(&title_row);

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
        let word_wrap_enabled = self
            .imp()
            .settings
            .borrow()
            .as_ref()
            .map(|settings| settings.boolean(SETTINGS_KEY_CUSTOM_USERSTYLE_WORD_WRAP))
            .unwrap_or(true);
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
        text_view.set_wrap_mode(if word_wrap_enabled {
            gtk::WrapMode::WordChar
        } else {
            gtk::WrapMode::None
        });
        text_view.add_css_class("code");
        text_view.add_css_class("userstyle-editor-view");
        buffer.set_text(&self.imp().saved_custom_userstyle_css.borrow());
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
        let word_wrap = gtk::CheckButton::with_label("Word wrap");
        word_wrap.set_active(word_wrap_enabled);
        let status = gtk::Label::new(Some("Ready"));
        status.set_xalign(0.0);
        status.add_css_class("dim-label");
        status.set_hexpand(true);
        controls.append(&live_preview);
        controls.append(&word_wrap);
        controls.append(&status);
        root.append(&controls);

        let diagnostics = gtk::Label::new(Some("CSS diagnostics: Ready"));
        diagnostics.set_xalign(0.0);
        diagnostics.add_css_class("dim-label");
        root.append(&diagnostics);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let import_css = gtk::Button::with_label("Import CSS...");
        import_css.add_css_class("flat");
        let export_css = gtk::Button::with_label("Export CSS...");
        export_css.add_css_class("flat");
        let save = gtk::Button::with_label("Save Changes");
        save.add_css_class("suggested-action");
        let close = gtk::Button::with_label("Discard Changes");
        close.add_css_class("flat");
        actions.append(&import_css);
        actions.append(&export_css);
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

        word_wrap.connect_toggled(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            text_view,
            move |toggle| {
                let enabled = toggle.is_active();
                text_view.set_wrap_mode(if enabled {
                    gtk::WrapMode::WordChar
                } else {
                    gtk::WrapMode::None
                });
                let settings = window.imp().settings.borrow().clone();
                if let Some(settings) = settings.as_ref() {
                    let _ = settings.set_boolean(SETTINGS_KEY_CUSTOM_USERSTYLE_WORD_WRAP, enabled);
                }
            }
        ));

        import_css.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[weak]
            buffer,
            #[weak]
            status,
            move |_| {
                let file_dialog = gtk::FileDialog::builder()
                    .title("Import Custom CSS")
                    .modal(true)
                    .build();
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("CSS files"));
                filter.add_pattern("*.css");
                filter.add_mime_type("text/css");
                filter.add_mime_type("text/plain");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                file_dialog.set_filters(Some(&filters));
                file_dialog.set_default_filter(Some(&filter));

                file_dialog.open(
                    Some(&dialog),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[weak(rename_to = window)]
                        window,
                        #[weak]
                        buffer,
                        #[weak]
                        status,
                        move |result: Result<gio::File, glib::Error>| {
                            match result {
                                Ok(file) => match file.load_contents(None::<&gio::Cancellable>) {
                                    Ok((contents, _)) => {
                                        let text =
                                            String::from_utf8_lossy(contents.as_ref()).to_string();
                                        buffer.set_text(&text);
                                        status.set_label("Imported CSS from file.");
                                    }
                                    Err(err) => {
                                        *window.imp().status_override.borrow_mut() =
                                            Some(format!("Import CSS failed: {err}"));
                                        window.render();
                                    }
                                },
                                Err(err) => {
                                    // Ignore user-cancel; report other failures.
                                    if err.matches(gio::IOErrorEnum::Cancelled) {
                                        return;
                                    }
                                    *window.imp().status_override.borrow_mut() =
                                        Some(format!("Import CSS failed: {err}"));
                                    window.render();
                                }
                            }
                        }
                    ),
                );
            }
        ));

        export_css.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[weak]
            buffer,
            #[weak]
            status,
            move |_| {
                let file_dialog = gtk::FileDialog::builder()
                    .title("Export Custom CSS")
                    .modal(true)
                    .initial_name("cardthropic-userstyle.css")
                    .build();
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("CSS files"));
                filter.add_pattern("*.css");
                filter.add_mime_type("text/css");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                file_dialog.set_filters(Some(&filters));
                file_dialog.set_default_filter(Some(&filter));

                file_dialog.save(
                    Some(&dialog),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[weak(rename_to = window)]
                        window,
                        #[weak]
                        buffer,
                        #[weak]
                        status,
                        move |result: Result<gio::File, glib::Error>| {
                            match result {
                                Ok(file) => {
                                    let text = buffer
                                        .text(&buffer.start_iter(), &buffer.end_iter(), true)
                                        .to_string();
                                    match file.replace_contents(
                                        text.as_bytes(),
                                        None,
                                        false,
                                        gio::FileCreateFlags::REPLACE_DESTINATION,
                                        None::<&gio::Cancellable>,
                                    ) {
                                        Ok(_) => {
                                            status.set_label("Exported CSS to file.");
                                        }
                                        Err(err) => {
                                            *window.imp().status_override.borrow_mut() =
                                                Some(format!("Export CSS failed: {err}"));
                                            window.render();
                                        }
                                    }
                                }
                                Err(err) => {
                                    if err.matches(gio::IOErrorEnum::Cancelled) {
                                        return;
                                    }
                                    *window.imp().status_override.borrow_mut() =
                                        Some(format!("Export CSS failed: {err}"));
                                    window.render();
                                }
                            }
                        }
                    ),
                );
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
            #[weak]
            unsaved_badge,
            #[strong]
            committed_css,
            move |buf| {
                let text = buf
                    .text(&buf.start_iter(), &buf.end_iter(), true)
                    .to_string();
                let dirty = text != *committed_css.borrow();
                unsaved_badge.set_visible(dirty);
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
            #[weak]
            unsaved_badge,
            move |_| {
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                window.apply_custom_userstyle(&text, true);
                *committed_css.borrow_mut() = text.clone();
                unsaved_badge.set_visible(false);
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
            #[weak]
            unsaved_badge,
            move |_| {
                let css = committed_css.borrow().clone();
                window.apply_custom_userstyle(&css, false);
                unsaved_badge.set_visible(false);
                dialog.close();
            }
        ));

        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            dialog,
            #[strong]
            allow_close,
            #[strong]
            committed_css,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                if allow_close.get() {
                    *window.imp().custom_userstyle_dialog.borrow_mut() = None;
                    return glib::Propagation::Proceed;
                }

                let current_css = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                let committed = committed_css.borrow().clone();
                if current_css == committed {
                    window.apply_custom_userstyle(&committed, false);
                    *window.imp().custom_userstyle_dialog.borrow_mut() = None;
                    return glib::Propagation::Proceed;
                }

                let confirm = gtk::Window::builder()
                    .title("Unsaved Custom CSS")
                    .transient_for(&dialog)
                    .modal(true)
                    .resizable(false)
                    .default_width(460)
                    .default_height(170)
                    .build();
                confirm.set_hide_on_close(true);

                let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
                let message = gtk::Label::new(Some(
                    "Custom CSS changed since opening. Save changes before closing?",
                ));
                message.set_wrap(true);
                message.set_xalign(0.0);
                message.set_margin_top(12);
                message.set_margin_bottom(12);
                message.set_margin_start(12);
                message.set_margin_end(12);
                content.append(&message);

                let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                actions.set_halign(gtk::Align::End);
                let cancel_button = gtk::Button::with_label("Cancel");
                cancel_button.add_css_class("flat");
                let discard_button = gtk::Button::with_label("Discard");
                discard_button.add_css_class("flat");
                let save_button = gtk::Button::with_label("Save");
                save_button.add_css_class("suggested-action");
                actions.append(&cancel_button);
                actions.append(&discard_button);
                actions.append(&save_button);
                content.append(&actions);

                cancel_button.connect_clicked(glib::clone!(
                    #[weak]
                    confirm,
                    move |_| {
                        confirm.close();
                    }
                ));
                discard_button.connect_clicked(glib::clone!(
                    #[weak(rename_to = window)]
                    window,
                    #[weak]
                    buffer,
                    #[weak]
                    dialog,
                    #[weak]
                    unsaved_badge,
                    #[weak]
                    confirm,
                    #[strong]
                    allow_close,
                    #[strong]
                    committed_css,
                    move |_| {
                        let _ = buffer; // keep weak buffer alive in closure captures symmetry
                        let css = committed_css.borrow().clone();
                        window.apply_custom_userstyle(&css, false);
                        unsaved_badge.set_visible(false);
                        allow_close.set(true);
                        confirm.close();
                        dialog.close();
                    }
                ));
                save_button.connect_clicked(glib::clone!(
                    #[weak(rename_to = window)]
                    window,
                    #[weak]
                    buffer,
                    #[weak]
                    dialog,
                    #[weak]
                    unsaved_badge,
                    #[weak]
                    confirm,
                    #[strong]
                    allow_close,
                    #[strong]
                    committed_css,
                    move |_| {
                        let text = buffer
                            .text(&buffer.start_iter(), &buffer.end_iter(), true)
                            .to_string();
                        window.apply_custom_userstyle(&text, true);
                        *committed_css.borrow_mut() = text;
                        unsaved_badge.set_visible(false);
                        allow_close.set(true);
                        confirm.close();
                        dialog.close();
                    }
                ));
                confirm.set_child(Some(&content));
                confirm.present();

                glib::Propagation::Stop
            }
        ));

        dialog.set_child(Some(&root));
        *self.imp().custom_userstyle_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }
}
