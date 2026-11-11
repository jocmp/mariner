//! `feed-prevent-repeated-images.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-prevent-repeated-images.xml",
        title: Some("Prensa sin censura"),
        site_url: Some("https://prensasincensura.com"),
        description: Some("Publicando lo que otros medios ignoran."),
        entry_id: Some("http://prensasincensura.com/?p=19030"),
        entry_title: Some(
            "El potencial de Venezuela en BRICS+ y la reconfiguración geopolítica mundial",
        ),
        entry_author: Some("Prensa sin censura-Periodismo Independiente al margen del capital"),
        entry_url: Some(
            "https://prensasincensura.com/2024/11/18/el-potencial-de-venezuela-en-brics-y-la-reconfiguracion-geopolitica-mundial-2/",
        ),
        categories: Some(&["Sin categoría"]),
        summary_prefix: Some(
            "El grupo BRICS (Brasil, Rusia, India, China y Sudáfrica), una coalición de econo",
        ),
        content_prefix: Some(
            "<p>El grupo BRICS (Brasil, Rusia, India, China y Sudáfrica), una coalición de ec",
        ),
        ..Case::DEFAULT
    }
    .check();
}
