//! Генерация PDF со встроенным шрифтом DejaVu (полная поддержка кириллицы).

use genpdf::{elements, style, Document, Element, SimplePageDecorator};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfBlock {
    pub heading: Option<String>, // имя спикера (жирным)
    pub time: Option<String>,    // таймкод
    pub body: String,            // текст реплики / абзац
}

pub fn save_pdf(title: &str, blocks: &[PdfBlock], path: &str) -> Result<(), String> {
    let regular =
        genpdf::fonts::FontData::new(include_bytes!("../../fonts/DejaVuSans.ttf").to_vec(), None)
            .map_err(|e| e.to_string())?;
    let bold =
        genpdf::fonts::FontData::new(include_bytes!("../../fonts/DejaVuSans-Bold.ttf").to_vec(), None)
            .map_err(|e| e.to_string())?;

    let family = genpdf::fonts::FontFamily {
        regular: regular.clone(),
        bold: bold.clone(),
        italic: regular,
        bold_italic: bold,
    };

    let mut doc = Document::new(family);
    doc.set_title(title);
    let mut deco = SimplePageDecorator::new();
    deco.set_margins(15);
    doc.set_page_decorator(deco);

    doc.push(elements::Paragraph::new(title).styled(style::Style::new().bold().with_font_size(17)));
    doc.push(elements::Break::new(1.0));

    for b in blocks {
        if let Some(h) = &b.heading {
            let mut p = elements::Paragraph::default();
            p.push_styled(h.clone(), style::Style::new().bold());
            if let Some(t) = &b.time {
                p.push_styled(
                    format!("   {t}"),
                    style::Style::new().with_color(style::Color::Rgb(140, 140, 140)),
                );
            }
            doc.push(p);
        }
        doc.push(elements::Paragraph::new(b.body.clone()));
        doc.push(elements::Break::new(0.6));
    }

    doc.render_to_file(path).map_err(|e| e.to_string())
}
