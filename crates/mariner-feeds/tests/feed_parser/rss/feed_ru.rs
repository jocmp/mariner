//! `feed-test-ru.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-ru.xml",
        title: Some("Аргументы и Факты"),
        site_url: Some("http://www.aif.ru/"),
        description: Some("Аргументы и Факты: объясняем, что происходит"),
        entry_title: Some("СМИ: собака в Таиланде спасла заживо похороненного младенца"),
        entry_url: Some(
            "http://www.aif.ru/incidents/smi_sobaka_v_tailande_spasla_zazhivo_pohoronennogo_mladenca",
        ),
        categories: Some(&["Происшествия"]),
        summary_prefix: Some(
            "15-летняя мать ребенка решила избавиться от него, побоявшись гнева родителей",
        ),
        ..Case::DEFAULT
    }
    .check();
}
