//! ブログモード(タグを意識しない文章)⇔コーディングモード(生HTML)の
//! 相互変換。
//!
//! 対応範囲(プロトタイプ): 見出し(`# `〜`### `)・箇条書き(`- `)・
//! 段落・強調(`**太字**`/`*斜体*`)のみ。コーディングモードで書かれた
//! 任意のHTML全般をブログモードへ完全に逆変換することは範囲外
//! (このコンバーターが生成したHTMLを開き直したときに崩れないことを
//! 必須要件とする、`CLAUDE.md`参照)。

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum EditorError {
    #[error("変換エラー: {0}")]
    Conversion(String),
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `**太字**` と `*斜体*` をインラインHTMLへ変換する(簡易実装、
/// ネスト・エスケープ記法は非対応)。
fn inline_to_html(text: &str) -> String {
    let escaped = escape_html(text);
    let mut out = String::new();
    let mut chars = escaped.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            if let Some(end) = find_marker(&mut chars, "**") {
                out.push_str("<strong>");
                out.push_str(&end);
                out.push_str("</strong>");
                continue;
            }
            out.push_str("**");
        } else if c == '*' {
            if let Some(end) = find_marker(&mut chars, "*") {
                out.push_str("<em>");
                out.push_str(&end);
                out.push_str("</em>");
                continue;
            }
            out.push('*');
        } else {
            out.push(c);
        }
    }
    out
}

fn find_marker(chars: &mut std::iter::Peekable<std::str::Chars>, marker: &str) -> Option<String> {
    let marker_chars: Vec<char> = marker.chars().collect();
    let mut buf = String::new();
    let mut matched = Vec::<char>::new();
    for c in chars.by_ref() {
        if c == marker_chars[0] {
            matched.push(c);
            if matched.len() == marker_chars.len() {
                return Some(buf);
            }
        } else {
            if !matched.is_empty() {
                buf.extend(matched.drain(..));
            }
            buf.push(c);
        }
    }
    None
}

/// ブログモードの文章をHTMLへ変換する。
pub fn blog_to_html(text: &str) -> Result<String, EditorError> {
    let mut html = String::new();
    let mut list_open = false;

    for block in text.split("\n\n") {
        let block = block.trim_end();
        if block.trim().is_empty() {
            continue;
        }

        let is_list_block = block.lines().all(|l| l.trim_start().starts_with("- "));
        if is_list_block {
            html.push_str("<ul>\n");
            for line in block.lines() {
                let item = line.trim_start().trim_start_matches("- ");
                html.push_str(&format!("  <li>{}</li>\n", inline_to_html(item)));
            }
            html.push_str("</ul>\n");
            list_open = false;
            continue;
        }
        let _ = list_open;

        if let Some(rest) = block.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", inline_to_html(rest.trim())));
        } else if let Some(rest) = block.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", inline_to_html(rest.trim())));
        } else if let Some(rest) = block.strip_prefix("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", inline_to_html(rest.trim())));
        } else {
            let joined = block.lines().collect::<Vec<_>>().join("<br>\n");
            html.push_str(&format!("<p>{}</p>\n", inline_to_html(&joined)));
        }
    }

    Ok(html)
}

fn unescape_html(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

fn inline_to_blog(html: &str) -> String {
    let text = html
        .replace("<strong>", "**")
        .replace("</strong>", "**")
        .replace("<em>", "*")
        .replace("</em>", "*")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    unescape_html(&text)
}

/// このコンバーターが生成した範囲のHTML(見出し/段落/箇条書き/強調のみ)を
/// ブログモードの文章へ逆変換する。想定外のタグは素通し(除去はしない)。
pub fn html_to_blog(html: &str) -> Result<String, EditorError> {
    let mut out = String::new();
    let mut rest = html;

    while let Some(start) = rest.find('<') {
        out.push_str(&inline_to_blog(&rest[..start]));
        rest = &rest[start..];
        let end = rest.find('>').ok_or_else(|| {
            EditorError::Conversion("閉じられていないタグがあります".to_string())
        })? + 1;
        let tag = &rest[..end];
        rest = &rest[end..];

        let close = format!("</{}>", tag_name(tag));
        if let Some(content_end) = rest.find(&close) {
            let content = &rest[..content_end];
            rest = &rest[content_end + close.len()..];
            match tag_name(tag).as_str() {
                "h1" => out.push_str(&format!("# {}\n\n", inline_to_blog(content).trim())),
                "h2" => out.push_str(&format!("## {}\n\n", inline_to_blog(content).trim())),
                "h3" => out.push_str(&format!("### {}\n\n", inline_to_blog(content).trim())),
                "p" => out.push_str(&format!("{}\n\n", inline_to_blog(content).trim())),
                "li" => out.push_str(&format!("- {}\n", inline_to_blog(content).trim())),
                "ul" | "ol" => {
                    out.push_str(&html_to_blog(content)?);
                    out.push('\n');
                }
                _ => out.push_str(&inline_to_blog(content)),
            }
        }
    }
    out.push_str(&inline_to_blog(rest));

    // 連続する空行を1つに畳む。
    let collapsed = out
        .split("\n\n")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(collapsed)
}

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_end_matches('>')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// `blog_to_html`のString引数版(Tauriコマンド/CLIコマンドいずれからも
/// そのまま使える薄いラッパー)。
pub fn editor_blog_to_html(text: String) -> Result<String, EditorError> {
    blog_to_html(&text)
}

/// `html_to_blog`のString引数版。
pub fn editor_html_to_blog(html: String) -> Result<String, EditorError> {
    html_to_blog(&html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_heading_and_paragraph() {
        let blog = "# タイトル\n\n本文の段落です。**強調**もできます。";
        let html = blog_to_html(blog).unwrap();
        assert!(html.contains("<h1>タイトル</h1>"));
        assert!(html.contains("<strong>強調</strong>"));

        let back = html_to_blog(&html).unwrap();
        assert!(back.contains("# タイトル"));
        assert!(back.contains("**強調**"));
    }

    #[test]
    fn roundtrip_list() {
        let blog = "- 項目1\n- 項目2";
        let html = blog_to_html(blog).unwrap();
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>項目1</li>"));

        let back = html_to_blog(&html).unwrap();
        assert!(back.contains("- 項目1"));
        assert!(back.contains("- 項目2"));
    }
}
