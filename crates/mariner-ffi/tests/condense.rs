//! Condense's document as the apps get it.

use mariner_ffi::{Block, CellKind, TextStyle, condense};

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[test]
fn spans_count_utf16_code_units() {
    let document = condense(
        "<p>😀 “Hi” <b>bold</b> <a href=\"/more\">more</a></p>".into(),
        "https://example.com/post".into(),
    )
    .unwrap();
    let [Block::Text { block }] = &document.blocks[..] else {
        panic!("expected one paragraph: {:?}", document.blocks);
    };
    let units = utf16(&block.text);
    let styled: Vec<(String, &TextStyle)> = block
        .spans
        .iter()
        .map(|span| {
            let range = span.start as usize..span.end as usize;
            (String::from_utf16(&units[range]).unwrap(), &span.style)
        })
        .collect();
    assert_eq!(
        styled,
        [
            ("bold".to_string(), &TextStyle::Bold),
            (
                "more".to_string(),
                &TextStyle::Link {
                    url: "https://example.com/more".into()
                }
            ),
        ]
    );
}

#[test]
fn keeps_nesting_and_places_table_cells() {
    let document = condense(
        "<ol><li><p>One</p><blockquote><p>Quoted</p></blockquote></li></ol>
         <table><tr><th colspan=2>Head</th></tr><tr><td>a</td><td>b</td></tr></table>"
            .into(),
        String::new(),
    )
    .unwrap();

    let Block::ListItem { block: item } = &document.blocks[0] else {
        panic!("expected a list item: {:?}", document.blocks[0]);
    };
    assert_eq!(item.number, Some(1));
    assert!(
        item.blocks
            .iter()
            .any(|block| matches!(block, Block::Quote { .. })),
        "{item:?}"
    );

    let table = document
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Table { block } => Some(block),
            _ => None,
        })
        .expect("a table");
    let cells: Vec<(u32, u32, u32, CellKind)> = table
        .cells
        .iter()
        .map(|cell| (cell.row, cell.col, cell.col_span, cell.kind))
        .collect();
    assert_eq!(
        cells,
        [
            (0, 0, 2, CellKind::Header),
            (1, 0, 1, CellKind::Data),
            (1, 1, 1, CellKind::Data),
        ]
    );
}

#[test]
fn reports_a_bad_base_url() {
    assert!(condense("<p>Hi</p>".into(), "not a url".into()).is_err());
}
