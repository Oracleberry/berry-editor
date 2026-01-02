//! Canvas Rendering Engine
//!
//! このモジュールだけがweb-sysの直接使用を許可されています。
//! 全てのCanvas描画操作はここに集約します。

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use crate::theme::{EditorTheme, RUSTROVER_DARCULA};

/// IntelliJ Darculaカラースキーム (Backward compatibility)
pub const COLOR_BACKGROUND: &str = "#1E1F22";  // Editor background (pixel-perfect)
pub const COLOR_FOREGROUND: &str = "#BCBEC4";  // Default text (pixel-perfect)
pub const COLOR_CURSOR: &str = "#BBBBBB";      // Caret
pub const COLOR_SELECTION: &str = "#214283";   // Selection
pub const COLOR_GUTTER_BG: &str = "#313335";   // Gutter background
pub const COLOR_GUTTER_FG: &str = "#4B5059";   // Line numbers (pixel-perfect)
pub const COLOR_LINE_HIGHLIGHT: &str = "#26282E"; // Current line (pixel-perfect)

/// フォント設定
pub const FONT_FAMILY: &str = "JetBrains Mono";
pub const FONT_SIZE: f64 = 13.0;  // RustRover actual size (smaller and crisper)
pub const LINE_HEIGHT: f64 = 20.0; // RustRover standard line height
pub const LETTER_SPACING: f64 = 0.0; // No extra spacing for sharp rendering

/// トークンの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Keyword,        // fn, pub, struct, let, mut (orange)
    KeywordImport,  // use, mod (blue)
    FunctionDef,    // function definition names (yellow)
    FunctionCall,   // function calls identifier() (bright blue)
    Type,           // String, usize, custom types (purple-pink)
    Module,         // module/crate names identifier:: (tan/orange)
    Identifier,     // variable/field names (white)
    String,         // string literals (green)
    Number,         // numeric literals (cyan)
    Comment,        // comments (gray)
    DocComment,     // /// doc comments (dark green)
    Attribute,      // #[derive] (yellow)
    Macro,          // println!, vec! (blue)
    Constant,       // CONSTANTS (purple)
    Punctuation,    // symbols, operators (white)
}

/// シンタックストークン
#[derive(Debug, Clone)]
struct SyntaxToken {
    text: String,
    kind: TokenKind,
}

/// Canvas描画エンジン
pub struct CanvasRenderer {
    context: CanvasRenderingContext2d,
    char_width_ascii: f64,
    char_width_wide: f64,
    line_height: f64,
    gutter_width: f64,
}

impl CanvasRenderer {
    /// Canvas要素から描画エンジンを作成
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        // ✅ Canvasコンテキストオプション設定（ネイティブアプリの質感に近づける）
        use wasm_bindgen::JsValue;
        let context_options = js_sys::Object::new();

        // alpha: false - 背景が透けないことを明示してテキスト品質向上
        let _ = js_sys::Reflect::set(
            &context_options,
            &JsValue::from_str("alpha"),
            &JsValue::from_bool(false)
        );

        // desynchronized: true - 低遅延モードでカーソルの動きをキビキビと
        let _ = js_sys::Reflect::set(
            &context_options,
            &JsValue::from_str("desynchronized"),
            &JsValue::from_bool(true)
        );

        let context = canvas
            .get_context_with_context_options("2d", &context_options)
            .map_err(|_| "Failed to get 2d context")?
            .ok_or("2d context is None")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

        // Retinaディスプレイ対応: devicePixelRatioでスケーリング
        let window = web_sys::window().ok_or("no global window")?;
        let dpr = window.device_pixel_ratio();

        #[cfg(debug_assertions)]
        web_sys::console::log_1(&format!("🎨 CanvasRenderer: DPR = {}, applying transform", dpr).into());

        // ✅ setTransform()を使用してスケーリングをリセットしてから設定
        // これにより、複数回呼ばれても累積されない
        // DPRが2.0なら、Canvasの内部解像度を2倍にしてCSSで元のサイズに戻す
        context
            .set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0)
            .map_err(|_| "Failed to set transform")?;

        // フォント品質設定（DPR適用後に設定）
        // Weight 300 (Light) - RustRoverの見本に合わせた軽量なフォント
        let font_string = format!("300 {}px '{}'", FONT_SIZE, FONT_FAMILY);

        #[cfg(debug_assertions)]
        web_sys::console::log_1(&format!("🎨 CanvasRenderer::new() - Setting font: {}", font_string).into());

        context.set_font(&font_string);

        // ✅ 設定直後にフォントを読み取って確認
        #[cfg(debug_assertions)]
        {
            let actual_font = context.font();
            web_sys::console::log_1(&format!("🎨 CanvasRenderer::new() - Font after set_font(): {}", actual_font).into());
        }

        // 高品質なテキストレンダリングを有効化
        context.set_image_smoothing_enabled(false); // Disable for sharper edges
        context.set_text_baseline("alphabetic");

        // テキストレンダリング品質の最適化
        // optimizeSpeed: エディタではピクセル整合性と描画速度を優先
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("fontKerning"),
            &JsValue::from_str("normal")
        );
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("textRendering"),
            &JsValue::from_str("optimizeSpeed")
        );

        // Letter spacing for ultra-crisp rendering
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("letterSpacing"),
            &JsValue::from_str(&format!("{}px", LETTER_SPACING))
        );

        // macOS/WebKit向けフォントスムージング最適化
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("imageSmoothingQuality"),
            &JsValue::from_str("high")
        );

        // 文字幅を実測
        let char_width_ascii = context
            .measure_text("M")
            .map_err(|_| "Failed to measure ASCII char")?
            .width();

        let char_width_wide = context
            .measure_text("あ")
            .map_err(|_| "Failed to measure wide char")?
            .width();

        Ok(Self {
            context,
            char_width_ascii,
            char_width_wide,
            line_height: LINE_HEIGHT,
            gutter_width: 55.0,
        })
    }

    /// ASCII文字幅を取得
    pub fn char_width_ascii(&self) -> f64 {
        self.char_width_ascii
    }

    /// 全角文字幅を取得
    pub fn char_width_wide(&self) -> f64 {
        self.char_width_wide
    }

    /// 行の高さを取得
    pub fn line_height(&self) -> f64 {
        self.line_height
    }

    /// ガター幅を取得
    pub fn gutter_width(&self) -> f64 {
        self.gutter_width
    }

    /// Canvas全体をクリア
    pub fn clear(&self, width: f64, height: f64) {
        let theme = EditorTheme::current();
        self.context.set_fill_style(&theme.bg_editor.into());
        self.context.fill_rect(0.0, 0.0, width, height);
    }

    /// 行番号ガターを描画
    pub fn draw_gutter(&self, start_line: usize, end_line: usize, height: f64) {
        // ガター背景
        self.context.set_fill_style(&COLOR_GUTTER_BG.into());
        self.context.fill_rect(0.0, 0.0, self.gutter_width, height);

        // 境界線
        self.context.set_stroke_style(&"#323232".into());
        self.context.begin_path();
        self.context.move_to(self.gutter_width, 0.0);
        self.context.line_to(self.gutter_width, height);
        self.context.stroke();

        // 行番号
        self.context.set_fill_style(&COLOR_GUTTER_FG.into());
        self.context.set_text_align("right");

        for line_num in start_line..end_line {
            // ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
            let y = ((line_num - start_line) as f64 * self.line_height + 15.0).round();
            let _ = self.context.fill_text(
                &(line_num + 1).to_string(),
                self.gutter_width - 10.0,
                y,
            );
        }

        self.context.set_text_align("left");
    }

    /// テキスト行を描画
    pub fn draw_line(&self, line_num: usize, y_offset: f64, text: &str, color: &str) {
        // ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
        let x = (self.gutter_width + 15.0).round();
        let y = (y_offset + 15.0).round();

        // ✅ 最初の行を描画する時だけフォントを確認（デバッグ用）
        #[cfg(debug_assertions)]
        if line_num == 0 {
            let current_font = self.context.font();
            web_sys::console::log_1(&format!("🎨 draw_line() - Current font: {}", current_font).into());
        }

        self.context.set_fill_style(&color.into());
        let _ = self.context.fill_text(text, x, y);
    }

    /// シンタックスハイライト付きでテキスト行を描画
    /// language: Some("rust") の場合はRustトークナイザーを使用、Noneの場合は単色
    pub fn draw_line_highlighted(&self, y_offset: f64, text: &str, theme: &EditorTheme, language: Option<&str>) {
        // ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
        let x_base = (self.gutter_width + 15.0).round();
        let y = (y_offset + 15.0).round();

        // 言語が指定されていない場合は単色で描画
        if language.is_none() {
            self.context.set_fill_style(&COLOR_FOREGROUND.into());
            let _ = self.context.fill_text(text, x_base, y);
            return;
        }

        // 言語に応じてトークナイズ
        let tokens = match language {
            Some("rust") => self.tokenize_rust(text),
            _ => {
                // サポートされていない言語は単色で描画
                self.context.set_fill_style(&COLOR_FOREGROUND.into());
                let _ = self.context.fill_text(text, x_base, y);
                return;
            }
        };

        let mut x_offset = 0.0;

        for token in tokens {
            let color = match token.kind {
                TokenKind::Keyword => theme.syntax_keyword,
                TokenKind::KeywordImport => theme.syntax_keyword_import,
                TokenKind::FunctionDef => theme.syntax_function_def,
                TokenKind::FunctionCall => theme.syntax_function_call,
                TokenKind::Type => theme.syntax_type,
                TokenKind::Module => theme.syntax_module,
                TokenKind::Identifier => theme.syntax_identifier,
                TokenKind::String => theme.syntax_string,
                TokenKind::Number => theme.syntax_number,
                TokenKind::Comment => theme.syntax_comment,
                TokenKind::DocComment => theme.syntax_doc_comment,
                TokenKind::Attribute => theme.syntax_attribute,
                TokenKind::Macro => theme.syntax_macro,
                TokenKind::Constant => theme.syntax_constant,
                TokenKind::Punctuation => theme.syntax_identifier,
            };

            self.context.set_fill_style(&color.into());
            // X座標も整数に丸める
            let _ = self.context.fill_text(&token.text, (x_base + x_offset).round(), y);

            // 次のトークンの位置を計算
            x_offset += self.measure_text(&token.text);
        }
    }

    /// Rustコードをトークンに分解
    fn tokenize_rust(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = line.chars().collect();
        let mut prev_token_was_fn = false;

        while current_pos < chars.len() {
            // ドキュメントコメント ///
            if current_pos + 2 < chars.len()
                && chars[current_pos] == '/'
                && chars[current_pos + 1] == '/'
                && chars[current_pos + 2] == '/' {
                let comment: String = chars[current_pos..].iter().collect();
                tokens.push(SyntaxToken {
                    text: comment,
                    kind: TokenKind::DocComment,
                });
                break;
            }

            // 通常のコメント //
            if current_pos + 1 < chars.len() && chars[current_pos] == '/' && chars[current_pos + 1] == '/' {
                let comment: String = chars[current_pos..].iter().collect();
                tokens.push(SyntaxToken {
                    text: comment,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // 文字列リテラル
            if chars[current_pos] == '"' {
                let mut end = current_pos + 1;
                while end < chars.len() && chars[end] != '"' {
                    if chars[end] == '\\' && end + 1 < chars.len() {
                        end += 2;
                    } else {
                        end += 1;
                    }
                }
                if end < chars.len() {
                    end += 1;
                }
                let string_lit: String = chars[current_pos..end].iter().collect();
                tokens.push(SyntaxToken {
                    text: string_lit,
                    kind: TokenKind::String,
                });
                current_pos = end;
                continue;
            }

            // 属性
            if chars[current_pos] == '#' && current_pos + 1 < chars.len() && chars[current_pos + 1] == '[' {
                let mut end = current_pos + 2;
                let mut bracket_count = 1;
                while end < chars.len() && bracket_count > 0 {
                    if chars[end] == '[' {
                        bracket_count += 1;
                    } else if chars[end] == ']' {
                        bracket_count -= 1;
                    }
                    end += 1;
                }
                let attr: String = chars[current_pos..end].iter().collect();
                tokens.push(SyntaxToken {
                    text: attr,
                    kind: TokenKind::Attribute,
                });
                current_pos = end;
                continue;
            }

            // 数値
            if chars[current_pos].is_ascii_digit() {
                let mut end = current_pos;
                while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.' || chars[end] == '_') {
                    end += 1;
                }
                let number: String = chars[current_pos..end].iter().collect();
                tokens.push(SyntaxToken {
                    text: number,
                    kind: TokenKind::Number,
                });
                current_pos = end;
                continue;
            }

            // 識別子/キーワード
            if chars[current_pos].is_alphabetic() || chars[current_pos] == '_' {
                let mut end = current_pos;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                let ident: String = chars[current_pos..end].iter().collect();

                // マクロ呼び出しチェック identifier!
                let is_macro = end < chars.len() && chars[end] == '!';
                if is_macro {
                    end += 1;
                    let macro_call: String = chars[current_pos..end].iter().collect();
                    tokens.push(SyntaxToken {
                        text: macro_call,
                        kind: TokenKind::Macro,
                    });
                    current_pos = end;
                    continue;
                }

                // モジュール名チェック identifier::
                let is_module = end + 1 < chars.len() && chars[end] == ':' && chars[end + 1] == ':';

                // 関数呼び出しチェック identifier(
                // 空白をスキップして (  をチェック
                let mut peek = end;
                while peek < chars.len() && chars[peek].is_whitespace() {
                    peek += 1;
                }
                let is_function_call = peek < chars.len() && chars[peek] == '(';

                // 関数定義名検出: `fn` の直後の識別子
                let kind = if prev_token_was_fn {
                    prev_token_was_fn = false;
                    TokenKind::FunctionDef
                } else if is_module {
                    TokenKind::Module
                } else if is_function_call {
                    TokenKind::FunctionCall
                } else {
                    match ident.as_str() {
                        // Import keywords (blue)
                        "use" | "mod" => TokenKind::KeywordImport,

                        // Regular keywords (orange)
                        "fn" | "pub" | "struct" | "enum" | "impl" | "trait" | "type" | "let" | "mut" |
                        "const" | "static" | "if" | "else" | "match" | "for" | "while" | "loop" |
                        "return" | "break" | "continue" | "crate" | "self" | "Self" |
                        "super" | "as" | "in" | "ref" | "move" | "unsafe" | "async" | "await" |
                        "dyn" | "where" | "true" | "false" => {
                            // `fn` キーワードを記憶
                            if ident == "fn" {
                                prev_token_was_fn = true;
                            }
                            TokenKind::Keyword
                        }

                        // 型
                        "String" | "str" | "usize" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" |
                        "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" | "bool" | "char" |
                        "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc" | "HashMap" | "HashSet" => TokenKind::Type,

                        // 大文字始まりは型と判断
                        _ if ident.chars().next().unwrap().is_uppercase() => TokenKind::Type,

                        // 全大文字は定数と判断
                        _ if ident.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit()) && ident.len() > 1 => TokenKind::Constant,

                        _ => TokenKind::Identifier,
                    }
                };

                tokens.push(SyntaxToken {
                    text: ident,
                    kind,
                });
                current_pos = end;
                continue;
            }

            // その他の文字（記号など）
            tokens.push(SyntaxToken {
                text: chars[current_pos].to_string(),
                kind: TokenKind::Punctuation,
            });
            current_pos += 1;
        }

        tokens
    }

    /// 指定座標にテキストを描画（IME未確定文字用）
    pub fn draw_text_at(&self, x: f64, y: f64, text: &str, color: &str) {
        // ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
        let x_rounded = x.round();
        let y_rounded = y.round();

        self.context.set_fill_style(&color.into());
        let _ = self.context.fill_text(text, x_rounded, y_rounded);
    }

    /// カーソルを描画（縦線）
    /// line_text: カーソルがある行のテキスト全体
    pub fn draw_cursor(&self, line: usize, col: usize, scroll_top: f64, line_text: &str) {
        // ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
        let x = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(line_text, col)).round();
        let y = (line as f64 * self.line_height - scroll_top).round();

        self.context.set_stroke_style(&COLOR_CURSOR.into());
        self.context.set_line_width(2.0);
        self.context.begin_path();
        self.context.move_to(x, y);
        self.context.line_to(x, y + self.line_height);
        self.context.stroke();
    }

    /// 選択範囲を描画
    /// get_line_text: 行番号から行のテキストを取得するクロージャ（日本語などマルチバイト文字の幅を正確に計算するため）
    pub fn draw_selection<F>(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        scroll_top: f64,
        get_line_text: F,
    ) where
        F: Fn(usize) -> String,
    {
        self.context.set_fill_style(&COLOR_SELECTION.into());

        if start_line == end_line {
            // 単一行の選択
            let line_text = get_line_text(start_line);
            // 日本語などマルチバイト文字の幅を正確に計算
            // ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
            let x_start = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&line_text, start_col)).round();
            let x_end = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&line_text, end_col)).round();
            let y = (start_line as f64 * self.line_height - scroll_top).round();

            self.context
                .fill_rect(x_start, y, x_end - x_start, self.line_height);
        } else {
            // 複数行の選択
            // 最初の行: start_colから行末まで
            let first_line_text = get_line_text(start_line);
            let first_line_chars: Vec<char> = first_line_text.chars().collect();
            // ピクセルグリッドに合わせて整数に丸める
            let x_start = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&first_line_text, start_col)).round();
            let x_end_first = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&first_line_text, first_line_chars.len())).round();
            let y_first = (start_line as f64 * self.line_height - scroll_top).round();

            self.context.fill_rect(
                x_start,
                y_first,
                x_end_first - x_start,
                self.line_height,
            );

            // 中間の行: 行全体を選択
            for line in (start_line + 1)..end_line {
                let middle_line_text = get_line_text(line);
                let middle_line_chars: Vec<char> = middle_line_text.chars().collect();
                // ピクセルグリッドに合わせて整数に丸める
                let x_start_middle = (self.gutter_width + 15.0).round();
                let x_end_middle = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&middle_line_text, middle_line_chars.len())).round();
                let y_middle = (line as f64 * self.line_height - scroll_top).round();

                self.context.fill_rect(
                    x_start_middle,
                    y_middle,
                    x_end_middle - x_start_middle,
                    self.line_height,
                );
            }

            // 最後の行: 行頭からend_colまで
            let last_line_text = get_line_text(end_line);
            // ピクセルグリッドに合わせて整数に丸める
            let x_start_last = (self.gutter_width + 15.0).round();
            let x_end_last = (self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&last_line_text, end_col)).round();
            let y_last = (end_line as f64 * self.line_height - scroll_top).round();

            self.context.fill_rect(
                x_start_last,
                y_last,
                x_end_last - x_start_last,
                self.line_height,
            );
        }
    }

    /// 文字列の幅を計算（ASCII + 全角混在対応）
    /// 実際のテキストから、指定された列位置までの幅を測定
    fn calculate_x_offset_from_text(&self, line_text: &str, col: usize) -> f64 {
        // 列位置までの文字列を取得
        let chars: Vec<char> = line_text.chars().collect();
        let end_col = col.min(chars.len());

        if end_col == 0 {
            return 0.0;
        }

        // カーソル位置までの文字列
        let text_up_to_cursor: String = chars[0..end_col].iter().collect();

        // 実際の幅を測定
        self.measure_text(&text_up_to_cursor)
    }

    /// 文字列の幅を計算（後方互換性のため残す、非推奨）
    #[allow(dead_code)]
    fn calculate_x_offset(&self, col: usize) -> f64 {
        // 簡易実装（ASCII幅のみ）
        col as f64 * self.char_width_ascii
    }

    /// 指定したテキストの実際の幅を測定
    pub fn measure_text(&self, text: &str) -> f64 {
        match self.context.measure_text(text) {
            Ok(metrics) => metrics.width(),
            Err(_) => 0.0,
        }
    }

    /// 現在のフォント設定を取得
    pub fn get_font(&self) -> String {
        self.context.font()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_constants() {
        assert_eq!(COLOR_BACKGROUND, "#1E1F22");  // Pixel-perfect editor background
        assert_eq!(COLOR_FOREGROUND, "#BCBEC4");  // Pixel-perfect default text color
    }

    #[test]
    fn test_font_constants() {
        assert_eq!(FONT_FAMILY, "JetBrains Mono");
        assert_eq!(FONT_SIZE, 13.0);
        assert_eq!(LINE_HEIGHT, 20.0);
        assert_eq!(LETTER_SPACING, 0.0);
    }
}
