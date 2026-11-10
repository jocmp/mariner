use mariner_condense::{
    ArticleDocument, Block, CellKind, CondenseError, Condenser, Style, TableBlock, TableSlot,
    TextBlock, TextKind, condense, condense_reader,
};

const BASE: &str = "https://example.com/posts/1";

#[test]
fn paragraphs_and_headings_split_into_text_blocks() {
    let doc = run("<h2>Title</h2><p>One</p><p>Two</p>");

    assert_eq!(texts(&doc), ["Title", "One", "Two"]);
    assert_eq!(
        styles(text(&doc.blocks[0])),
        [(Style::Heading { level: 2 }, "Title")]
    );
}

#[test]
fn heading_levels_one_through_six() {
    let doc = run("<h1>a</h1><h2>b</h2><h3>c</h3><h4>d</h4><h5>e</h5><h6>f</h6>");

    let levels: Vec<u8> = doc
        .blocks
        .iter()
        .map(|block| match &text(block).spans[0].style {
            Style::Heading { level } => *level,
            other => panic!("expected a heading, got {other:?}"),
        })
        .collect();
    assert_eq!(levels, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn generic_containers_do_not_create_blocks() {
    let doc = run(
        "<div><section><article>Hello <custom-tag>there</custom-tag></article></section></div>",
    );

    assert_eq!(texts(&doc), ["Hello there"]);
}

#[test]
fn sibling_containers_keep_their_text_apart() {
    let doc = run("<div>One</div><div>Two</div>");

    assert_eq!(texts(&doc), ["One", "Two"]);
}

#[test]
fn inline_styles() {
    let doc = run(
        "<p><b>b</b> <strong>s</strong> <i>i</i> <em>e</em> <u>u</u> <s>x</s> <del>d</del> \
         <sup>1</sup> <sub>2</sub> <tt>t</tt> <kbd>k</kbd> <code>c</code> \
         <font face=\"Georgia\">f</font></p>",
    );

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [
            (Style::Bold, "b"),
            (Style::Bold, "s"),
            (Style::Italic, "i"),
            (Style::Italic, "e"),
            (Style::Underline, "u"),
            (Style::Strikethrough, "x"),
            (Style::Strikethrough, "d"),
            (Style::Superscript, "1"),
            (Style::Subscript, "2"),
            (Style::Monospace, "t"),
            (Style::Monospace, "k"),
            (Style::Code, "c"),
            (
                Style::Font {
                    face: "Georgia".into()
                },
                "f"
            ),
        ]
    );
}

#[test]
fn spans_are_utf8_byte_ranges() {
    let doc = run("<p>caf\u{e9} <b>na\u{ef}ve</b></p>");
    let block = text(&doc.blocks[0]);

    let span = &block.spans[0];
    assert_eq!(span.range(), 6..12);
    assert_eq!(&block.text[span.range()], "na\u{ef}ve");
}

#[test]
fn overlapping_styles_nest() {
    let doc = run("<p><b>bold <i>both</i></b> plain</p>");

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [(Style::Bold, "bold both"), (Style::Italic, "both")]
    );
}

#[test]
fn links_resolve_against_the_base_url() {
    let doc =
        run("<p><a href=\"../about\">About</a> <a href=\"mailto:me@example.com\">Mail</a></p>");

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [
            (link("https://example.com/about"), "About"),
            (link("mailto:me@example.com"), "Mail"),
        ]
    );
}

#[test]
fn unresolvable_links_keep_the_raw_href() {
    let doc = run("<p><a href=\"http://[bad\">Bad</a></p>");

    assert_eq!(styles(text(&doc.blocks[0])), [(link("http://[bad"), "Bad")]);
}

#[test]
fn styles_carry_over_block_boundaries() {
    let doc = run("<a href=\"/x\"><p>One</p><p>Two</p></a>");

    let expected = link("https://example.com/x");
    assert_eq!(styles(text(&doc.blocks[0])), [(expected.clone(), "One")]);
    assert_eq!(styles(text(&doc.blocks[1])), [(expected, "Two")]);
}

#[test]
fn whitespace_collapses_like_a_browser() {
    let doc = run("<p>\n  Hello,\t\t<b> big </b>\n world!   </p>");

    assert_eq!(texts(&doc), ["Hello, big world!"]);
}

#[test]
fn preformatted_text_keeps_whitespace() {
    let doc = run(
        "<pre>  line one\n    line two  \n</pre><pre><code>fn main() {\n    run();\n}</code></pre>",
    );

    assert_eq!(
        texts(&doc),
        ["  line one\n    line two", "fn main() {\n    run();\n}"]
    );
    assert_eq!(text(&doc.blocks[0]).kind, TextKind::Preformatted);
    assert_eq!(text(&doc.blocks[1]).kind, TextKind::Code);
    assert!(text(&doc.blocks[1]).spans.is_empty());
}

#[test]
fn line_breaks_stay_within_a_block() {
    let doc = run("<p>One<br>Two <br/> Three<br></p>");

    assert_eq!(texts(&doc), ["One\nTwo\nThree"]);
}

#[test]
fn span_inline_styles() {
    let doc = run(
        "<p><span style=\"font-weight: bold\">a</span> <span style=\"font-weight:700\">b</span> \
         <span style=\"font-style: italic\">c</span> <span style=\"font-style: oblique\">d</span></p>",
    );

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [
            (Style::Bold, "a"),
            (Style::Bold, "b"),
            (Style::Italic, "c"),
            (Style::Italic, "d"),
        ]
    );
}

#[test]
fn display_none_is_dropped() {
    let doc = run(
        "<p>Shown <span style=\"display: none\">hidden</span>text</p><div style=\"DISPLAY:NONE\"><p>Gone</p></div>",
    );

    assert_eq!(texts(&doc), ["Shown text"]);
}

#[test]
fn ruby_annotations_are_dropped() {
    let doc = run("<p><ruby>\u{6f22}<rp>(</rp><rt>kan</rt><rp>)</rp></ruby></p>");

    assert_eq!(texts(&doc), ["\u{6f22}"]);
}

#[test]
fn scripts_and_styles_are_dropped() {
    let doc = run("<p>Text</p><script>alert(1)</script><style>p {}</style><noscript>no</noscript>");

    assert_eq!(texts(&doc), ["Text"]);
}

#[test]
fn image_sources_from_src_srcset_and_lazy_attributes() {
    let doc = run(
        "<img src=\"data:image/gif;base64,R0lGOD\" data-src=\"/real.jpg\" width=\"640\" height=\"480\" \
         srcset=\"/small.jpg 320w, /large.jpg 1280w, /retina.jpg 2x\">",
    );

    let image = image(&doc.blocks[0]);
    let urls: Vec<&str> = image.sources.iter().map(|s| s.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://example.com/real.jpg",
            "https://example.com/small.jpg",
            "https://example.com/large.jpg",
            "https://example.com/retina.jpg",
        ]
    );
    assert_eq!(
        (image.sources[0].width, image.sources[0].height),
        (Some(640), Some(480))
    );
    assert_eq!(image.sources[2].srcset_width, Some(1280));
    assert_eq!(image.sources[3].density, Some(2.0));
}

#[test]
fn background_images_become_images() {
    let doc = run("<div style=\"background-image: url('/Hero.JPG')\"></div>");

    assert_eq!(
        image(&doc.blocks[0]).sources[0].url,
        "https://example.com/Hero.JPG"
    );
}

#[test]
fn bare_images_use_alt_text_as_caption() {
    let doc = run("<p>Before</p><img src=\"/a.png\" alt=\"A <b>bold</b>   claim\"><p>After</p>");

    assert_eq!(caption(&doc.blocks[1]), Some("A bold claim"));
    assert_eq!(texts(&doc), ["Before", "After"]);
}

#[test]
fn linked_images_keep_the_link() {
    let doc = run("<a href=\"/full.png\"><img src=\"/thumb.png\"></a>");

    assert_eq!(
        image(&doc.blocks[0]).link.as_deref(),
        Some("https://example.com/full.png")
    );
}

#[test]
fn figures_use_their_figcaption() {
    let doc = run(
        "<figure><img src=\"/a.png\" alt=\"alt text\"><figcaption>The <i>real</i> caption</figcaption></figure><p>Next</p>",
    );

    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(caption(&doc.blocks[0]), Some("The real caption"));
    let caption = image(&doc.blocks[0]).caption.as_ref().unwrap();
    assert_eq!(styles(caption), [(Style::Italic, "real")]);
}

#[test]
fn nested_figures_keep_their_own_captions() {
    let doc = run(
        "<figure><figure><img src=\"/a.png\"><figcaption>Inner</figcaption></figure>\
         <img src=\"/b.png\"><figcaption>Outer</figcaption></figure>",
    );

    assert_eq!(caption(&doc.blocks[0]), Some("Inner"));
    assert_eq!(caption(&doc.blocks[1]), Some("Outer"));
}

#[test]
fn figure_captions_attach_to_an_image_just_outside_the_figure() {
    let doc = run("<img src=\"/a.png\"><figure><figcaption>Caption</figcaption></figure>");

    assert_eq!(doc.blocks.len(), 1);
    assert_eq!(caption(&doc.blocks[0]), Some("Caption"));
}

#[test]
fn picture_sources_join_the_image() {
    let doc =
        run("<picture><source srcset=\"/a.avif 1x, /a@2x.avif 2x\"><img src=\"/a.jpg\"></picture>");

    let urls: Vec<&str> = image(&doc.blocks[0])
        .sources
        .iter()
        .map(|s| s.url.as_str())
        .collect();
    assert_eq!(
        urls,
        [
            "https://example.com/a.jpg",
            "https://example.com/a.avif",
            "https://example.com/a@2x.avif",
        ]
    );
}

#[test]
fn unordered_lists() {
    let doc = run("<ul><li>One</li><li>Two</li></ul>");

    assert_eq!(doc.blocks.len(), 2);
    for (block, expected) in doc.blocks.iter().zip(["One", "Two"]) {
        let Block::ListItem(item) = block else {
            panic!("expected a list item")
        };
        assert_eq!(item.number, None);
        assert_eq!(text(&item.blocks[0]).text, expected);
    }
}

#[test]
fn ordered_list_numbering() {
    let numbers = |html: &str| -> Vec<Option<i32>> {
        run(html)
            .blocks
            .iter()
            .map(|block| match block {
                Block::ListItem(item) => item.number,
                other => panic!("expected a list item, got {other:?}"),
            })
            .collect()
    };

    assert_eq!(
        numbers("<ol><li>a<li>b<li>c</ol>"),
        [Some(1), Some(2), Some(3)]
    );
    assert_eq!(
        numbers("<ol start=\"5\"><li>a<li>b</ol>"),
        [Some(5), Some(6)]
    );
    assert_eq!(
        numbers("<ol><li>a<li value=\"10\">b<li>c</ol>"),
        [Some(1), Some(10), Some(11)]
    );
    assert_eq!(
        numbers("<ol reversed><li>a<li>b<li>c</ol>"),
        [Some(3), Some(2), Some(1)]
    );
}

#[test]
fn nested_lists_live_inside_their_item() {
    let doc = run("<ul><li>Parent<ol><li>Child</li></ol></li></ul>");

    assert_eq!(doc.blocks.len(), 1);
    let Block::ListItem(parent) = &doc.blocks[0] else {
        panic!("expected a list item")
    };
    assert_eq!(text(&parent.blocks[0]).text, "Parent");
    let Block::ListItem(child) = &parent.blocks[1] else {
        panic!("expected a nested item")
    };
    assert_eq!(child.number, Some(1));
    assert_eq!(text(&child.blocks[0]).text, "Child");
}

#[test]
fn quotes_hold_text_and_their_citation() {
    let doc = run("<blockquote cite=\"/source\"><p>Quoted</p><p>More</p></blockquote>");

    let Block::Quote(quote) = &doc.blocks[0] else {
        panic!("expected a quote")
    };
    assert_eq!(quote.cite.as_deref(), Some("https://example.com/source"));
    assert_eq!(quote.blocks.len(), 2);
}

#[test]
fn quotes_split_around_media() {
    let doc = run("<blockquote><p>Before</p><img src=\"/a.png\"><p>After</p></blockquote>");

    assert_eq!(doc.blocks.len(), 3);
    assert!(matches!(&doc.blocks[0], Block::Quote(q) if text(&q.blocks[0]).text == "Before"));
    assert!(matches!(doc.blocks[1], Block::Image(_)));
    assert!(matches!(&doc.blocks[2], Block::Quote(q) if text(&q.blocks[0]).text == "After"));
}

#[test]
fn tables_honor_section_order_and_spans() {
    let doc = run("<table>\
           <tfoot><tr><td>F1</td><td>F2</td></tr></tfoot>\
           <tbody><tr><td rowspan=\"2\">B1</td><td>B2</td></tr><tr><td>B3</td></tr></tbody>\
           <thead><tr><th colspan=\"2\">Head</th></tr></thead>\
         </table>");

    let table = table(&doc.blocks[0]);
    assert_eq!((table.rows, table.cols), (4, 2));
    assert_eq!(cell_text(table, 0, 0), "Head");
    assert_eq!(table.cell(0, 0).unwrap().kind, CellKind::Header);
    assert_eq!(
        table.slot(0, 1),
        Some(&TableSlot::Covered { row: 0, col: 0 })
    );
    assert_eq!(cell_text(table, 1, 0), "B1");
    assert_eq!(cell_text(table, 1, 1), "B2");
    assert_eq!(
        table.slot(2, 0),
        Some(&TableSlot::Covered { row: 1, col: 0 })
    );
    assert_eq!(cell_text(table, 2, 1), "B3");
    assert_eq!(cell_text(table, 3, 0), "F1");
}

#[test]
fn table_layout_gives_span_aware_grid_cells() {
    let doc = run(
        "<table><tr><td colspan=2>A</td><td>B</td></tr><tr><td>C</td><td>D</td><td>E</td></tr></table>",
    );

    let layout: Vec<(usize, usize, usize, usize)> = table(&doc.blocks[0])
        .layout()
        .iter()
        .map(|cell| (cell.row, cell.col, cell.row_span, cell.col_span))
        .collect();
    assert_eq!(
        layout,
        [
            (0, 0, 1, 2),
            (0, 2, 1, 1),
            (1, 0, 1, 1),
            (1, 1, 1, 1),
            (1, 2, 1, 1)
        ]
    );
}

#[test]
fn right_to_left_tables_are_mirrored() {
    let doc = run(
        "<table dir=\"rtl\"><tr><td>First</td><td>Second</td></tr><tr><td colspan=2>Wide</td></tr></table>",
    );

    let table = table(&doc.blocks[0]);
    assert_eq!(cell_text(table, 0, 0), "Second");
    assert_eq!(cell_text(table, 0, 1), "First");
    assert_eq!(cell_text(table, 1, 0), "Wide");
}

#[test]
fn single_row_and_single_column_tables_become_plain_content() {
    let row = run("<table><tr><td>A</td><td>B</td></tr></table>");
    let column = run("<table><tr><td>A</td></tr><tr><td>B</td></tr></table>");

    assert_eq!(texts(&row), ["A", "B"]);
    assert_eq!(texts(&column), ["A", "B"]);
}

#[test]
fn youtube_embeds_become_videos() {
    for src in [
        "https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0",
        "//www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
        "https://youtu.be/dQw4w9WgXcQ",
        "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
    ] {
        let doc = run(&format!(
            "<iframe src=\"{src}\" width=\"560\" height=\"315\"></iframe>"
        ));

        let Block::Video(video) = &doc.blocks[0] else {
            panic!("expected a video for {src}")
        };
        let source = &video.sources[0];
        assert_eq!(source.url, "https://www.youtube.com/embed/dQw4w9WgXcQ");
        assert_eq!(
            source.link.as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
        assert_eq!(
            source.thumbnail.as_deref(),
            Some("https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg")
        );
        assert_eq!((source.width, source.height), (Some(560), Some(315)));
    }
}

#[test]
fn other_iframes_are_dropped() {
    let doc = run("<p>Text</p><iframe src=\"https://ads.example.com/frame\"></iframe>");

    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn video_and_audio_sources() {
    let doc = run(
        "<video poster=\"/poster.jpg\"><source src=\"/clip.webm\" type=\"video/webm\"><source src=\"/clip.mp4\"></video>\
         <audio src=\"/episode.mp3\" type=\"audio/mpeg\"></audio>",
    );

    let Block::Video(video) = &doc.blocks[0] else {
        panic!("expected a video")
    };
    assert_eq!(video.sources.len(), 2);
    assert_eq!(video.sources[0].mime_type.as_deref(), Some("video/webm"));
    assert_eq!(
        video.sources[0].thumbnail.as_deref(),
        Some("https://example.com/poster.jpg")
    );
    let Block::Audio(audio) = &doc.blocks[1] else {
        panic!("expected audio")
    };
    assert_eq!(audio.sources[0].url, "https://example.com/episode.mp3");
    assert_eq!(audio.sources[0].mime_type.as_deref(), Some("audio/mpeg"));
}

#[test]
fn ids_resolve_to_their_top_level_block() {
    let doc = run(
        "<h2 id=\"intro\">Intro</h2><p>See <span id=\"note\">this</span></p>\
         <ul id=\"list\"><li id=\"item\">Item <a id=\"deep\">x</a></li></ul>\
         <table><tr><td id=\"cell\">a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>",
    );

    assert_eq!(doc.anchors["intro"], 0);
    assert_eq!(doc.anchors["note"], 1);
    assert_eq!(doc.anchors["list"], 2);
    assert_eq!(doc.anchors["item"], 2);
    assert_eq!(doc.anchors["deep"], 2);
    assert_eq!(doc.anchors["cell"], 3);
    assert!(doc.blocks[0].ids().contains("intro"));
    assert!(matches!(doc.block_for_anchor("note"), Some(Block::Text(t)) if t.text == "See this"));
}

#[test]
fn image_urls_include_nested_images() {
    let doc = run(
        "<img src=\"/top.png\"><ul><li><img src=\"/list.png\"></li></ul>\
         <table><tr><td><img src=\"/cell.png\"></td><td>x</td></tr><tr><td>y</td><td>z</td></tr></table>",
    );

    let urls: Vec<&str> = doc.image_urls.iter().map(String::as_str).collect();
    assert_eq!(
        urls,
        [
            "https://example.com/cell.png",
            "https://example.com/list.png",
            "https://example.com/top.png",
        ]
    );
}

#[test]
fn block_cap_stops_conversion() {
    let html = "<p>x</p>".repeat(10);
    let doc = Condenser::new()
        .max_blocks(3)
        .condense(&html, BASE)
        .unwrap();

    assert_eq!(doc.blocks.len(), 3);
    let truncation = doc.truncation.expect("truncation");
    assert_eq!((truncation.blocks, truncation.chars), (3, 3));
}

#[test]
fn char_cap_stops_conversion() {
    let html = "<p>12345</p>".repeat(10);
    let doc = Condenser::new()
        .max_chars(12)
        .condense(&html, BASE)
        .unwrap();

    assert_eq!(texts(&doc), ["12345", "12345"]);
    assert_eq!(doc.truncation.map(|t| t.chars), Some(10));
}

#[test]
fn documents_under_the_caps_are_not_truncated() {
    assert_eq!(run("<p>Short</p>").truncation, None);
}

#[test]
fn large_real_world_page_stays_within_the_caps() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mercury-parser/tests/fixtures/www.reddit.com--image.html"
    );
    let html = std::fs::read_to_string(path).unwrap();

    let doc = condense(&html, "https://www.reddit.com/").unwrap();
    let mut blocks = 0;
    for block in &doc.blocks {
        block.visit(&mut |_| blocks += 1);
    }
    assert!(blocks <= mariner_condense::DEFAULT_MAX_BLOCKS);

    let small = Condenser::new()
        .max_blocks(25)
        .condense(&html, "https://www.reddit.com/")
        .unwrap();
    assert_eq!(small.truncation.map(|t| t.blocks), Some(25));
}

#[test]
fn invalid_base_urls_fail() {
    assert!(matches!(
        condense("<p>x</p>", "not a url"),
        Err(CondenseError::InvalidBaseUrl { .. })
    ));
    assert!(condense("<p><a href=\"https://a.example/\">x</a></p>", "").is_ok());
}

#[test]
fn reads_from_a_reader() {
    let doc = condense_reader("<p>From a reader</p>".as_bytes(), BASE).unwrap();

    assert_eq!(texts(&doc), ["From a reader"]);
}

#[test]
fn serializes_to_json_and_back() {
    let doc = run(
        "<h1 id=\"t\">T</h1><p><b>x</b></p><img src=\"/a.png\"><table><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>",
    );

    let json = serde_json::to_string(&doc).unwrap();
    let parsed: ArticleDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, doc);
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("\"style\":\"bold\""));
}

fn run(html: &str) -> ArticleDocument {
    condense(html, BASE).unwrap()
}

fn texts(doc: &ArticleDocument) -> Vec<&str> {
    doc.blocks
        .iter()
        .filter_map(|block| match block {
            Block::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect()
}

fn text(block: &Block) -> &TextBlock {
    match block {
        Block::Text(text) => text,
        other => panic!("expected text, got {other:?}"),
    }
}

fn image(block: &Block) -> &mariner_condense::ImageBlock {
    match block {
        Block::Image(image) => image,
        other => panic!("expected an image, got {other:?}"),
    }
}

fn table(block: &Block) -> &TableBlock {
    match block {
        Block::Table(table) => table,
        other => panic!("expected a table, got {other:?}"),
    }
}

fn caption(block: &Block) -> Option<&str> {
    image(block)
        .caption
        .as_ref()
        .map(|caption| caption.text.as_str())
}

fn cell_text(table: &TableBlock, row: usize, col: usize) -> &str {
    let cell = table
        .cell(row, col)
        .unwrap_or_else(|| panic!("no cell at ({row}, {col})"));
    &text(&cell.blocks[0]).text
}

fn styles(block: &TextBlock) -> Vec<(Style, &str)> {
    block
        .spans
        .iter()
        .map(|span| (span.style.clone(), &block.text[span.range()]))
        .collect()
}

fn link(url: &str) -> Style {
    Style::Link {
        url: url.to_string(),
    }
}
