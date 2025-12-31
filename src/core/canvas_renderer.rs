//! Canvas Rendering Engine
//!
//! このモジュールだけがweb-sysの直接使用を許可されています。
//! 全てのCanvas描画操作はここに集約します。

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// IntelliJ Darculaカラースキーム
pub const COLOR_BACKGROUND: &str = "#1E1E1E";
pub const COLOR_FOREGROUND: &str = "#A9B7C6";
pub const COLOR_CURSOR: &str = "#FFFFFF";
pub const COLOR_SELECTION: &str = "#214283";
pub const COLOR_GUTTER_BG: &str = "#313335";
pub const COLOR_GUTTER_FG: &str = "#606366";
pub const COLOR_LINE_HIGHLIGHT: &str = "#323232";

/// フォント設定
pub const FONT_FAMILY: &str = "JetBrains Mono";
pub const FONT_SIZE: f64 = 13.0;
pub const LINE_HEIGHT: f64 = 20.0;

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
        let context = canvas
            .get_context("2d")
            .map_err(|_| "Failed to get 2d context")?
            .ok_or("2d context is None")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

        // フォント設定
        context.set_font(&format!("{}px '{}'", FONT_SIZE, FONT_FAMILY));

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
        self.context.set_fill_style(&COLOR_BACKGROUND.into());
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
            let y = (line_num - start_line) as f64 * self.line_height + 15.0;
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
        let x = self.gutter_width + 15.0; // 左パディング
        let y = y_offset + 15.0; // ベースライン調整

        self.context.set_fill_style(&color.into());
        let _ = self.context.fill_text(text, x, y);
    }

    /// 指定座標にテキストを描画（IME未確定文字用）
    pub fn draw_text_at(&self, x: f64, y: f64, text: &str, color: &str) {
        self.context.set_fill_style(&color.into());
        let _ = self.context.fill_text(text, x, y);
    }

    /// カーソルを描画（縦線）
    /// line_text: カーソルがある行のテキスト全体
    pub fn draw_cursor(&self, line: usize, col: usize, scroll_top: f64, line_text: &str) {
        let x = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(line_text, col);
        let y = line as f64 * self.line_height - scroll_top;

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
            let x_start = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&line_text, start_col);
            let x_end = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&line_text, end_col);
            let y = start_line as f64 * self.line_height - scroll_top;

            self.context
                .fill_rect(x_start, y, x_end - x_start, self.line_height);
        } else {
            // 複数行の選択
            // 最初の行: start_colから行末まで
            let first_line_text = get_line_text(start_line);
            let first_line_chars: Vec<char> = first_line_text.chars().collect();
            let x_start = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&first_line_text, start_col);
            let x_end_first = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&first_line_text, first_line_chars.len());
            let y_first = start_line as f64 * self.line_height - scroll_top;

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
                let x_start_middle = self.gutter_width + 15.0;
                let x_end_middle = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&middle_line_text, middle_line_chars.len());
                let y_middle = line as f64 * self.line_height - scroll_top;

                self.context.fill_rect(
                    x_start_middle,
                    y_middle,
                    x_end_middle - x_start_middle,
                    self.line_height,
                );
            }

            // 最後の行: 行頭からend_colまで
            let last_line_text = get_line_text(end_line);
            let x_start_last = self.gutter_width + 15.0;
            let x_end_last = self.gutter_width + 15.0 + self.calculate_x_offset_from_text(&last_line_text, end_col);
            let y_last = end_line as f64 * self.line_height - scroll_top;

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
        assert_eq!(COLOR_BACKGROUND, "#1E1E1E");
        assert_eq!(COLOR_FOREGROUND, "#A9B7C6");
    }

    #[test]
    fn test_font_constants() {
        assert_eq!(FONT_FAMILY, "JetBrains Mono");
        assert_eq!(FONT_SIZE, 13.0);
        assert_eq!(LINE_HEIGHT, 20.0);
    }
}
