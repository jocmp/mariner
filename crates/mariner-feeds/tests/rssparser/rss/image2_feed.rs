//! `feed-test-image-2.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-image-2.xml",
        title: Some("F.C. Barcelona"),
        site_url: Some("https://www.mundodeportivo.com/futbol/fc-barcelona"),
        description: Some("F.C. Barcelona"),
        entry_id: Some("f5c42a9c-78d9-11e9-a24c-645e8f5d185b"),
        entry_title: Some("Martens: “Estoy feliz y quiero seguir en el Barça”"),
        entry_url: Some(
            "https://www.mundodeportivo.com/futbol/fc-barcelona/20190517/462298326260/martens-estoy-feliz-y-quiero-seguir-en-el-barca.html",
        ),
        categories: Some(&["F.C. Barcelona"]),
        summary_prefix: Some(
            "El diario francés l’Equipe se hizo eco ayer de una noticia de impacto: el Olympi",
        ),
        ..Case::DEFAULT
    }
    .check();
}
