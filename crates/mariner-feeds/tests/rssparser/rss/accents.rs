//! `feed-test-accents.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-accents.xml",
        charset: Some("ISO-8859-1"),
        title: Some("UOL Noticias"),
        site_url: Some("http://noticias.uol.com.br/"),
        description: Some("Últimas Notícias"),
        entry_title: Some("Artur Jorge não é mais técnico do Botafogo"),
        entry_url: Some(
            "https://noticias.uol.com.br/ultimas-noticias/afp/2025/01/03/artur-jorge-nao-e-mais-tecnico-do-botafogo.htm",
        ),
        summary_prefix: Some(
            "O português Artur Jorge não continuará como treinador do Botafogo, anunciou nest",
        ),
        ..Case::DEFAULT
    }
    .check();
}
