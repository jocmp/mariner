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
fn generic_containers_flow_into_the_surrounding_text() {
    let doc = run(
        "<div><section><article>Hello <custom-tag>there</custom-tag></article></section></div>",
    );
    assert_eq!(texts(&doc), ["Hello there"]);

    let doc = run("before<div>inside</div><hr><section>after</section>");
    assert_eq!(texts(&doc), ["beforeinsideafter"]);
}

#[test]
fn paragraphs_inside_containers_still_split() {
    let doc = run("<div>one<p>two</p>three</div>");

    assert_eq!(texts(&doc), ["one", "two", "three"]);
}

#[test]
fn inline_styles() {
    let doc = run(
        "<p><b>b</b> <strong>s</strong> <i>i</i> <em>e</em> <cite>c</cite> <dfn>d</dfn> \
         <u>u</u> <s>x</s> <sup>1</sup> <sub>2</sub> <tt>t</tt> <code>c</code> \
         <font face=\"Georgia\">f</font></p>",
    );

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [
            (Style::Bold, "b"),
            (Style::Bold, "s"),
            (Style::Italic, "i"),
            (Style::Italic, "e"),
            (Style::Italic, "c"),
            (Style::Italic, "d"),
            (Style::Underline, "u"),
            (Style::Strikethrough, "x"),
            (Style::Superscript, "1"),
            (Style::Subscript, "2"),
            (Style::Monospace, "t"),
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
fn other_inline_tags_are_unstyled() {
    let doc = run("<p><del>d</del> <ins>i</ins> <kbd>k</kbd> <samp>s</samp> <mark>m</mark></p>");

    assert_eq!(texts(&doc), ["d i k s m"]);
    assert!(text(&doc.blocks[0]).spans.is_empty());
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
    let doc = run(
        "<p><a href=\"../about\">About</a> <a href=\"mailto:me@example.com\">Mail</a> \
         <a href=\"\">Here</a> <a href=\"//cdn.example.com/x\">Cdn</a></p>",
    );

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [
            (link("https://example.com/about"), "About"),
            (link("mailto:me@example.com"), "Mail"),
            (link(BASE), "Here"),
            (link("https://cdn.example.com/x"), "Cdn"),
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
fn non_breaking_spaces_collapse_too() {
    let doc = run("<p>one\u{a0}\u{a0}two \u{a0}three</p>");

    assert_eq!(texts(&doc), ["one two three"]);
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
    assert_eq!(
        styles(text(&doc.blocks[1])),
        [(Style::Code, "fn main() {\n    run();\n}")]
    );
}

#[test]
fn line_breaks_stay_within_a_block() {
    let doc = run("<p>One<br>Two <br/> Three</p><p><br>Leading</p>");

    assert_eq!(texts(&doc), ["One\nTwo\nThree", "\nLeading"]);
}

#[test]
fn span_inline_styles() {
    let doc = run(
        "<p><span style=\"font-weight: bold\">a</span> <span style=\"font-style: italic\">b</span> \
         <span style=\"font-style: oblique\">c</span> <span style=\"font-weight:700\">d</span></p>",
    );

    assert_eq!(
        styles(text(&doc.blocks[0])),
        [
            (Style::Bold, "a"),
            (Style::Italic, "b"),
            (Style::Italic, "c")
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
fn image_sources_come_from_srcset_then_data_src_then_src() {
    let doc = run(
        "<img src=\"data:image/gif;base64,R0lGOD\" data-src=\"/real.jpg\" width=\"640\" height=\"480\" \
         srcset=\"/small.jpg 320w, /large.jpg 1280w, /retina.jpg 2x\">",
    );

    let image = image(&doc.blocks[0]);
    let urls: Vec<&str> = image.sources.iter().map(|s| s.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://example.com/small.jpg",
            "https://example.com/large.jpg",
            "https://example.com/retina.jpg",
            "https://example.com/real.jpg",
            "data:image/gif;base64,R0lGOD",
        ]
    );
    assert_eq!(image.sources[1].srcset_width, Some(1280));
    assert_eq!(image.sources[2].density, Some(2.0));
    assert_eq!(
        (image.sources[3].width, image.sources[3].height),
        (Some(640), Some(480))
    );
}

#[test]
fn image_dimensions_need_both_plain_integers() {
    let size = |html: &str| {
        let doc = run(html);
        let source = &image(&doc.blocks[0]).sources[0];
        (source.width, source.height)
    };

    assert_eq!(
        size("<img src=\"/a.jpg\" width=\"100\" height=\"50\">"),
        (Some(100), Some(50))
    );
    assert_eq!(size("<img src=\"/a.jpg\" width=\"100\">"), (None, None));
    assert_eq!(
        size("<img src=\"/a.jpg\" width=\"100px\" height=\"50\">"),
        (None, None)
    );
}

#[test]
fn background_images_on_img_are_sources() {
    let doc = run(
        "<img style=\"background-image: url('/Hero.JPG')\"><div style=\"background-image: url(/div.jpg)\">text</div>",
    );

    assert_eq!(
        image(&doc.blocks[0]).sources[0].url,
        "https://example.com/Hero.JPG"
    );
    assert_eq!(texts(&doc), ["text"]);
}

#[test]
fn bare_images_use_alt_text_as_caption() {
    let doc = run("<p>Before</p><img src=\"/a.png\" alt=\"A <b>bold</b>   claim\"><p>After</p>");

    assert_eq!(caption(&doc.blocks[1]), Some("A bold claim"));
    assert_eq!(texts(&doc), ["Before", "After"]);
}

#[test]
fn images_split_the_paragraph_around_them() {
    let doc = run("<p>Text before <img src=\"/inline.jpg\"> text after</p>");

    assert_eq!(texts(&doc), ["Text before", "text after"]);
    assert!(matches!(doc.blocks[1], Block::Image(_)));
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
fn figures_take_their_caption_from_their_own_text() {
    let doc = run(
        "<figure><img src=\"/a.png\" alt=\"alt text\"><figcaption>The <i>real</i> caption</figcaption></figure><p>Next</p>",
    );

    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(caption(&doc.blocks[0]), Some("The real caption"));
    let figcaption = image(&doc.blocks[0]).caption.as_ref().unwrap();
    assert_eq!(styles(figcaption), [(Style::Italic, "real")]);

    let doc = run("<figure><img src=\"/a.png\"><div><p>Credit</p><p>More</p></div></figure>");
    assert_eq!(caption(&doc.blocks[0]), Some("Credit"));

    let doc = run("<figure><img src=\"/a.png\" alt=\"Alt\"></figure>");
    assert_eq!(caption(&doc.blocks[0]), None);
}

#[test]
fn figures_are_one_image_from_every_image_inside() {
    let doc = run(
        "<figure><figure><img src=\"/a.png\"><figcaption>Inner</figcaption></figure>\
         <img src=\"/b.png\"><figcaption>Outer</figcaption></figure>",
    );

    assert_eq!(doc.blocks.len(), 1);
    let urls: Vec<&str> = image(&doc.blocks[0])
        .sources
        .iter()
        .map(|s| s.url.as_str())
        .collect();
    assert_eq!(
        urls,
        ["https://example.com/a.png", "https://example.com/b.png"]
    );
    assert_eq!(caption(&doc.blocks[0]), Some("Outer"));
}

#[test]
fn figures_without_images_are_dropped_unless_they_embed_video() {
    let doc = run(
        "<figure><blockquote>Pull quote</blockquote><figcaption>Cap</figcaption></figure>\
         <figure><iframe src=\"https://www.youtube.com/embed/dQw4w9WgXcQ\"></iframe></figure><p>after</p>",
    );

    assert_eq!(doc.blocks.len(), 2);
    assert!(matches!(doc.blocks[0], Block::Video(_)));
    assert_eq!(texts(&doc), ["after"]);
}

#[test]
fn figures_keep_an_enclosing_link() {
    let doc = run("<a href=\"/x\"><figure><img src=\"/a.png\"></figure></a>");

    assert_eq!(
        image(&doc.blocks[0]).link.as_deref(),
        Some("https://example.com/x")
    );
}

#[test]
fn picture_sources_are_ignored() {
    let doc = run("<picture><source srcset=\"/a.avif\"><img src=\"/a.jpg\"></picture>");

    let urls: Vec<&str> = image(&doc.blocks[0])
        .sources
        .iter()
        .map(|s| s.url.as_str())
        .collect();
    assert_eq!(urls, ["https://example.com/a.jpg"]);
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
fn ordered_items_are_numbered_by_position() {
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
        numbers("<ol start=\"5\" reversed><li>a<li value=\"9\">b</ol>"),
        [Some(1), Some(2)]
    );
}

#[test]
fn only_list_items_count_inside_a_list() {
    let doc = run("<ul>stray<li>item</li></ul><li>orphan</li>");

    assert_eq!(doc.blocks.len(), 2);
    assert!(
        matches!(&doc.blocks[0], Block::ListItem(item) if text(&item.blocks[0]).text == "item")
    );
    assert_eq!(texts(&doc), ["orphan"]);
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
    assert_eq!(quote.cite.as_deref(), Some("/source"));
    assert_eq!(quote.blocks.len(), 2);
}

#[test]
fn quotes_split_around_everything_but_text() {
    let doc = run(
        "<blockquote><p>Before</p><img src=\"/a.png\"><p>Middle</p><ul><li>Item</li></ul>\
         <blockquote><p>Inner</p></blockquote><p>After</p></blockquote>",
    );

    let kinds: Vec<&str> = doc
        .blocks
        .iter()
        .map(|block| match block {
            Block::Quote(_) => "quote",
            Block::Image(_) => "image",
            Block::ListItem(_) => "item",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["quote", "image", "quote", "item", "quote", "quote"]);
}

#[test]
fn media_inside_a_list_item_in_a_quote_stays_in_the_item() {
    let doc = run("<blockquote><ul><li>item <img src=\"/q.jpg\"> more</li></ul></blockquote>");

    assert_eq!(doc.blocks.len(), 1);
    let Block::ListItem(item) = &doc.blocks[0] else {
        panic!("expected a list item")
    };
    assert_eq!(item.blocks.len(), 3);
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
fn rowspan_zero_spans_to_the_last_row() {
    let doc = run(
        "<table><tr><td rowspan=\"0\">Tall</td><td>a</td></tr><tr><td>b</td></tr><tr><td>c</td></tr></table>",
    );

    let table = table(&doc.blocks[0]);
    assert_eq!(table.cell(0, 0).unwrap().row_span, 3);
    assert_eq!(cell_text(table, 2, 1), "c");
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
fn table_captions_are_dropped() {
    let doc = run(
        "<table><caption>Cap</caption><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>",
    );

    assert_eq!(doc.blocks.len(), 1);
    assert!(matches!(doc.blocks[0], Block::Table(_)));
}

#[test]
fn youtube_embeds_become_videos() {
    for (src, url) in [
        (
            "https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0",
            "https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0",
        ),
        (
            "//www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
            "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
        ),
        (
            "https://youtu.be/dQw4w9WgXcQ",
            "https://youtu.be/dQw4w9WgXcQ",
        ),
        (
            "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
        ),
    ] {
        let doc = run(&format!("<iframe src=\"{src}\" width=\"560\"></iframe>"));

        let Block::Video(video) = &doc.blocks[0] else {
            panic!("expected a video for {src}")
        };
        let source = &video.sources[0];
        assert_eq!(source.url, url);
        assert_eq!(
            source.link.as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
        assert_eq!(
            source.thumbnail.as_deref(),
            Some("https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg")
        );
        assert_eq!((source.width, source.height), (Some(560), Some(360)));
    }
}

#[test]
fn other_iframes_are_dropped() {
    let doc = run(
        "<p>Text</p><iframe src=\"https://player.vimeo.com/video/1\"></iframe>\
         <iframe data-src=\"https://www.youtube.com/embed/dQw4w9WgXcQ\"></iframe>",
    );

    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn video_and_audio_sources() {
    let doc = run(
        "<video poster=\"/poster.jpg\" width=\"640\" height=\"360\"><source src=\"/clip.webm\" type=\"video/webm\"><source src=\"/clip.mp4\">fallback</video>\
         <audio src=\"/ignored.mp3\"><source src=\"/episode.mp3\" type=\"audio/mpeg\"></audio>",
    );

    assert_eq!(doc.blocks.len(), 2);
    let Block::Video(video) = &doc.blocks[0] else {
        panic!("expected a video")
    };
    assert_eq!(video.sources.len(), 2);
    let source = &video.sources[0];
    assert_eq!(
        source.link.as_deref(),
        Some("https://example.com/clip.webm")
    );
    assert_eq!(source.mime_type.as_deref(), Some("video/webm"));
    assert_eq!(
        source.thumbnail.as_deref(),
        Some("https://example.com/poster.jpg")
    );
    assert_eq!((source.width, source.height), (Some(640), Some(360)));
    let Block::Audio(audio) = &doc.blocks[1] else {
        panic!("expected audio")
    };
    assert_eq!(audio.sources.len(), 1);
    assert_eq!(audio.sources[0].url, "https://example.com/episode.mp3");
    assert_eq!(audio.sources[0].mime_type.as_deref(), Some("audio/mpeg"));
}

#[test]
fn ids_resolve_to_their_top_level_block() {
    let doc = run(
        "<h2 id=\"intro\">Intro</h2><p>See <span id=\"note\">this</span></p>\
         <ul id=\"list\"><li id=\"item\">Item <a id=\"deep\">x</a></li></ul>\
         <table><tr><td id=\"cell\">a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>\
         <figure id=\"figure\"><img id=\"image\" src=\"/a.png\"><figcaption id=\"caption\">c</figcaption></figure>",
    );

    assert_eq!(doc.anchors["intro"], 0);
    assert_eq!(doc.anchors["note"], 1);
    assert_eq!(doc.anchors["list"], 2);
    assert_eq!(doc.anchors["item"], 2);
    assert_eq!(doc.anchors["deep"], 2);
    assert_eq!(doc.anchors["cell"], 3);
    assert_eq!(doc.anchors["figure"], 4);
    assert_eq!(doc.anchors["image"], 4);
    assert_eq!(doc.anchors["caption"], 4);
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
    // The counts include the block that went over the cap.
    assert_eq!((truncation.blocks, truncation.chars), (4, 4));
}

#[test]
fn char_cap_stops_conversion() {
    let html = "<p>12345</p>".repeat(10);
    let doc = Condenser::new()
        .max_chars(12)
        .condense(&html, BASE)
        .unwrap();

    assert_eq!(texts(&doc), ["12345", "12345"]);
    assert_eq!(doc.truncation.map(|t| (t.blocks, t.chars)), Some((3, 15)));
}

#[test]
fn documents_under_the_caps_are_not_truncated() {
    assert_eq!(run("<p>Short</p>").truncation, None);
    let exactly = "<p>x</p>".repeat(3);
    let doc = Condenser::new()
        .max_blocks(3)
        .condense(&exactly, BASE)
        .unwrap();
    assert_eq!(doc.truncation, None);
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
        .max_blocks(5)
        .condense(&html, "https://www.reddit.com/")
        .unwrap();
    assert_eq!(small.blocks.len(), 5);
    assert_eq!(small.truncation.map(|t| t.blocks), Some(6));
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
        "<h1 id=\"t\">T</h1><p><b>x</b> <a href=\"/l\">l</a></p><img src=\"/a.png\"><table><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>",
    );

    let json = serde_json::to_string(&doc).unwrap();
    let parsed: ArticleDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, doc);
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("{\"style\":\"bold\",\"start\":0,\"end\":1}"));
    assert!(
        json.contains(
            "{\"style\":\"link\",\"url\":\"https://example.com/l\",\"start\":2,\"end\":3}"
        )
    );
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
