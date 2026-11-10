//! The first entry of each fixture in `fixtures/rssparser/`, checked
//! against the values those fixtures were written to test, kept only where
//! they agree with the reference output. Text is compared trimmed, and
//! `summary` and `content` by their first line.

use mariner_feeds::{Feed, parse_with_charset};

#[test]
fn matches_expected_values() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rssparser");
    let mut failures = Vec::new();
    for case in CASES {
        let bytes = std::fs::read(format!("{fixtures}/{}", case.fixture)).unwrap();
        let feed = parse_with_charset(&bytes, case.charset).unwrap();
        failures.extend(
            case.differences(&feed)
                .into_iter()
                .map(|difference| format!("{}: {difference}", case.fixture)),
        );
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

struct Case {
    fixture: &'static str,
    charset: Option<&'static str>,
    title: Option<&'static str>,
    site_url: Option<&'static str>,
    description: Option<&'static str>,
    entry_id: Option<&'static str>,
    entry_title: Option<&'static str>,
    entry_author: Option<&'static str>,
    entry_url: Option<&'static str>,
    categories: Option<&'static [&'static str]>,
    summary_prefix: Option<&'static str>,
    content_prefix: Option<&'static str>,
}

impl Case {
    const DEFAULT: Case = Case {
        fixture: "",
        charset: None,
        title: None,
        site_url: None,
        description: None,
        entry_id: None,
        entry_title: None,
        entry_author: None,
        entry_url: None,
        categories: None,
        summary_prefix: None,
        content_prefix: None,
    };

    fn differences(&self, feed: &Feed) -> Vec<String> {
        let entry = feed.entries.first().cloned().unwrap_or_default();
        let mut differences = Vec::new();
        let mut equal = |name: &str, expected: Option<&str>, actual: &Option<String>| {
            if let Some(expected) = expected
                && actual.as_deref().map(str::trim) != Some(expected)
            {
                differences.push(format!("{name}: expected {expected:?}, got {actual:?}"));
            }
        };
        equal("title", self.title, &feed.title);
        equal("site_url", self.site_url, &feed.site_url);
        equal("description", self.description, &feed.description);
        equal("entry id", self.entry_id, &entry.id);
        equal("entry title", self.entry_title, &entry.title);
        equal("entry author", self.entry_author, &entry.author);
        equal("entry url", self.entry_url, &entry.url);
        let mut starts = |name: &str, expected: Option<&str>, actual: &Option<String>| {
            if let Some(expected) = expected
                && !actual
                    .as_deref()
                    .is_some_and(|actual| actual.trim().starts_with(expected))
            {
                differences.push(format!("{name}: expected {expected:?}..., got {actual:?}"));
            }
        };
        starts("entry summary", self.summary_prefix, &entry.summary);
        starts("entry content", self.content_prefix, &entry.content);
        if let Some(expected) = self.categories
            && entry.categories != expected
        {
            differences.push(format!(
                "entry categories: expected {expected:?}, got {:?}",
                entry.categories
            ));
        }
        differences
    }
}

const CASES: &[Case] = &[
    Case {
        fixture: "feed-test-atom-category-attribute.xml",
        title: Some("Slashdot"),
        site_url: Some("http://pubsubhubbub.appspot.com/"),
        description: Some("News for nerds, stuff that matters"),
        entry_id: Some(
            "https://yro.slashdot.org/story/23/05/28/2213240/automakers-ask-judge-to-block-pending-enforcement-of-massachusetts-right-to-repair-law?utm_source=atom1.0mainlinkanon&utm_medium=feed",
        ),
        entry_title: Some(
            "Automakers Ask Judge to Block Pending Enforcement of Massachusetts' Right-to-Repair Law",
        ),
        entry_author: Some("EditorDavid"),
        entry_url: Some(
            "https://yro.slashdot.org/story/23/05/28/2213240/automakers-ask-judge-to-block-pending-enforcement-of-massachusetts-right-to-repair-law?utm_source=atom1.0mainlinkanon&utm_medium=feed",
        ),
        categories: Some(&["government"]),
        summary_prefix: Some(
            "\"Beginning next Thursday, Massachusetts Attorney General Andrea Joy Campbell pla",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-atom-content-html.xml",
        title: Some("Jake Wharton"),
        site_url: Some("https://jakewharton.com/"),
        entry_id: Some("https://jakewharton.com/the-state-of-managing-state-with-compose"),
        entry_title: Some("The state of managing state (with Compose)"),
        entry_url: Some("https://code.cash.app/the-state-of-managing-state-with-compose"),
        content_prefix: Some(
            "This post was published externally on Cash App Code Blog. Read it at https://cod",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-test-example.xml",
        title: Some("Example Feed"),
        site_url: Some("http://example.org/"),
        description: Some("A subtitle."),
        entry_id: Some("urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a"),
        entry_title: Some("Atom-Powered Robots Run Amok"),
        entry_author: Some("John Doe"),
        entry_url: Some("http://example.org/2003/12/13/atom03.html"),
        summary_prefix: Some("Some text."),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-feed-content.xml",
        title: Some("GrapheneOS changelog"),
        site_url: Some("https://grapheneos.org/LICENSE.txt"),
        entry_id: Some("https://grapheneos.org/releases#2025041100"),
        entry_title: Some("2025041100"),
        entry_url: Some("https://grapheneos.org/releases#2025041100"),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-feed-created-updated.xml",
        title: Some("tonsky.me"),
        site_url: Some("https://tonsky.me/"),
        entry_id: Some("https://tonsky.me/blog/unicode/"),
        entry_title: Some(
            "The Absolute Minimum Every Software Developer Must Know About Unicode in 2023 (Still No Excuses!)",
        ),
        entry_author: Some("Nikita Prokopov"),
        entry_url: Some("https://tonsky.me/blog/unicode/"),
        summary_prefix: Some("Modern extension to classic 2003 article by Joel Spolsky"),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-image-query-params.xml",
        title: Some("9to5Google"),
        site_url: Some("https://9to5google.com/"),
        description: Some("Google news, Pixel, Android, Home, Chrome OS, more"),
        entry_id: Some("https://9to5google.com/2024/09/02/labor-day-android-game-app-deals/"),
        entry_title: Some(
            "Labor Day Android game and app price drops: SOULVARS, Bloons TD 6, Space Grunts 2, more",
        ),
        entry_author: Some("9to5Toys"),
        entry_url: Some("https://9to5toys.com/2024/09/02/labor-day-android-game-app-deals/"),
        categories: Some(&["Deals"]),
        summary_prefix: Some(
            "<div class=\"feat-image\"><img src=\"https://9to5google.com/wp-content/uploads/site",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-relative-image.xml",
        title: Some("Crystal's Wobsite: Articles"),
        site_url: Some("https://crystalwobsite.gay"),
        entry_id: Some("https://crystalwobsite.gay/posts/2025-10-03-fluentflame.html"),
        entry_title: Some("Working on Fluentflame Reader"),
        entry_url: Some("https://crystalwobsite.gay/posts/2025-10-03-fluentflame"),
        summary_prefix: Some(
            "Small blogpost because I don’t feel like writing a long one, but I felt I",
        ),
        content_prefix: Some(
            "<p>Small blogpost because I don’t feel like writing a long one, but I felt I",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-replies-link-example.xml",
        title: Some("9to5Linux"),
        site_url: Some("https://9to5linux.com/"),
        description: Some("Linux news, reviews, tutorials, and more"),
        entry_id: Some("https://9to5linux.com/?p=23839"),
        entry_title: Some(
            "Fwupd 1.9.24 Firmware Updater Adds Support for Capsule-on-Disk for Dell Systems",
        ),
        entry_author: Some("Marius Nestor"),
        entry_url: Some(
            "https://9to5linux.com/fwupd-1-9-24-adds-support-for-capsule-on-disk-for-dell-systems",
        ),
        categories: Some(&[
            "App",
            "News",
            "firmware update",
            "firmware upgrade",
            "fwupd",
            "Fwupd 1.9.24",
            "Linux firmware",
        ]),
        summary_prefix: Some(
            "<p>fwupd 1.9.24 Linux firmware updater is now available for download with suppor",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-self-link-example.xml",
        title: Some("Simon Willison's Weblog"),
        site_url: Some("http://simonwillison.net/"),
        entry_id: Some(
            "http://simonwillison.net/2023/Jul/29/a-steering-council-notice-about-pep-703/#atom-everything",
        ),
        entry_title: Some(
            "A Steering Council notice about PEP 703 (Making the Global Interpreter Lock Optional\n            in CPython)",
        ),
        entry_url: Some(
            "http://simonwillison.net/2023/Jul/29/a-steering-council-notice-about-pep-703/#atom-everything",
        ),
        summary_prefix: Some("<p><a"),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-atom-test.xml",
        title: Some("The Verge -  All Posts"),
        site_url: Some("https://www.theverge.com/"),
        entry_id: Some(
            "https://www.theverge.com/2023/5/26/23739273/google-sonos-smart-speaker-patent-lawsuit-ruling",
        ),
        entry_title: Some("Sonos wins $32.5 million patent infringement victory over Google"),
        entry_author: Some("Chris Welch"),
        entry_url: Some(
            "https://www.theverge.com/2023/5/26/23739273/google-sonos-smart-speaker-patent-lawsuit-ruling",
        ),
        content_prefix: Some("<figure>"),
        ..Case::DEFAULT
    },
    Case {
        fixture: "atom-feed-youtube.xml",
        title: Some("Digital Foundry"),
        site_url: Some("https://www.youtube.com/channel/UC9PBzalIcEQCsiIkq36PyUA"),
        entry_id: Some("yt:video:W_S7iKZZdHw"),
        entry_title: Some("Digital Foundry Games of 2024: The Oliver Mackenzie Collection!"),
        entry_author: Some("Digital Foundry"),
        entry_url: Some("https://www.youtube.com/watch?v=W_S7iKZZdHw"),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-kibty-town.json",
        title: Some("xyzeva's blog"),
        site_url: Some("https://kibty.town/"),
        description: Some("random thoughts and other stuff"),
        entry_title: Some("gaining access to anyones browser without them even visiting a website"),
        entry_url: Some("https://kibty.town/blog/arc/"),
        content_prefix: Some(
            "<p>we start at the homepage of arc. where i first landed when i first heard of i",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-npr-world.json",
        title: Some("World"),
        site_url: Some(
            "https://www.npr.org/sections/world/?utm_medium=JSONFeed&utm_campaign=world",
        ),
        description: Some(
            "NPR world news, international art and culture, world business and financial markets, world economy, and global trends in health, science and technology. Subscribe to the World Story of the Day podcast and RSS feed.",
        ),
        entry_title: Some("Eggs and Bananas: Life after a Russian prison"),
        entry_url: Some(
            "https://www.npr.org/2024/08/26/1198913145/eggs-and-bananas-life-after-a-russian-prison?utm_medium=JSONFeed&utm_campaign=world",
        ),
        summary_prefix: Some(
            "It's been more than three weeks since the U.S. and Russia completed the largest ",
        ),
        content_prefix: Some(
            "<p>It's been more than three weeks since the U.S. and Russia completed the large",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-rdf-distrowatch.xml",
        title: Some("DistroWatch.com: DistroWatch Weekly"),
        site_url: Some("https://distrowatch.com/"),
        description: Some("Latest news on Linux distributions and BSD projects"),
        entry_title: Some("DistroWatch Weekly, Issue 1084"),
        entry_url: Some("https://distrowatch.com/weekly.php?issue=20240819"),
        summary_prefix: Some(
            "The DistroWatch Weekly news feed is brought to you by <a href=\"https://www.tuxed",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-rdf-test.xml",
        title: Some("Slashdot"),
        site_url: Some("https://slashdot.org/"),
        description: Some("News for nerds, stuff that matters"),
        entry_title: Some(
            "Google Play Will No Longer Pay To Discover Vulnerabilities In Popular Android Apps",
        ),
        entry_author: Some("BeauHD"),
        entry_url: Some(
            "https://tech.slashdot.org/story/24/08/22/2042250/google-play-will-no-longer-pay-to-discover-vulnerabilities-in-popular-android-apps?utm_source=rss1.0mainlinkanon&utm_medium=feed",
        ),
        summary_prefix: Some(
            "Android Authority's Mishaal Rahman reports: Security vulnerabilities are lurking",
        ),
        ..Case::DEFAULT
    },
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
    },
    Case {
        fixture: "feed-test-audio.xml",
        title: Some("Stuff You Should Know"),
        site_url: Some("https://www.howstuffworks.com"),
        description: Some(
            "If you've ever wanted to know about champagne, satanism, the Stonewall Uprising, chaos theory, LSD, El Nino, true crime and Rosa Parks, then look no further. Josh and Chuck have you covered.",
        ),
        entry_id: Some("c41042ae-5460-11e8-b38c-5f5faf9dd0c5"),
        entry_title: Some("SYSK Selects: How Gold Works"),
        summary_prefix: Some(
            "As of early 2013, only 161,00 metric tons of gold had been mined in the entire h",
        ),
        content_prefix: Some(
            "<p>As of early 2013, only 161,00 metric tons of gold had been mined in the entir",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-bing-image.xml",
        title: Some("madrid - BingNews"),
        site_url: Some("https://www.bing.com:443/news/search?q=madrid&format=rss"),
        description: Some("Search results"),
        entry_title: Some(
            "Real Madrid, Barcelona and Juventus 'threaten clubs withdrawing from Super League\n                with legal action'",
        ),
        entry_url: Some(
            "http://www.bing.com/news/apiclick.aspx?ref=FexRss&aid=&tid=12BE3CE268B0484F92DD9828C685E325&url=https%3a%2f%2fwww.dailymail.co.uk%2fsport%2ffootball%2farticle-9554407%2fReal-Madrid-Barcelona-Juventus-threaten-clubs-withdrawing-Super-League-legal-action.html&c=4438839993362862681&mkt=it-it",
        ),
        summary_prefix: Some(
            "The three European giants are refusing to let the plans die and have",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-char-escape.xml",
        title: Some("NYT > Health"),
        site_url: Some("https://www.nytimes.com/section/health"),
        entry_id: Some(
            "https://www.nytimes.com/2021/06/11/science/drought-las-vegas-grass-mars.html",
        ),
        entry_title: Some("Where the Grass is Greener, Except When It’s ‘Nonfunctional Turf’"),
        entry_author: Some("Alan Burdick"),
        entry_url: Some(
            "https://www.nytimes.com/2021/06/11/science/drought-las-vegas-grass-mars.html",
        ),
        summary_prefix: Some(
            "Plus, mammoths in Vegas, watermelon snow, Miami’s looming sea wall and more in t",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-charset.xml",
        title: Some("Lørdagsrådet"),
        site_url: Some("https://radio.nrk.no/podkast/loerdagsraadet"),
        description: Some(
            "Hver uke blar tre (mer eller mindre) kvalifiserte rådgivere i sitt livserfaringsarkiv for å hjelpe deg. Lurer du på om du skal gjøre det slutt, er naboen din en tyrann eller er du håpløst forelsket? Vi skal prøve å gi deg gode råd. <a href=\"https://radio.nrk.no/podkast/loerdagsraadet?utm_source=thirdparty&utm_medium=rss&utm_content=podcastseries%3Aloerdagsraadet\">Hør alle episodene først i appen NRK Radio</a>",
        ),
        entry_id: Some("l_e6338d4f-2777-43c4-b38d-4f2777e3c4de"),
        entry_title: Some("Høydepunkter fra 2022 - nr.11"),
        summary_prefix: Some(
            "Høydepunkter fra 2022 - nr.11. <a href=\"https://radio.nrk.no/podkast/loerdagsraa",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-comment.xml",
        title: Some("Hacker News"),
        site_url: Some("https://news.ycombinator.com/"),
        description: Some("Links for the intellectually curious, ranked by readers."),
        entry_url: Some("https://gonzoknows.com/posts/GPTZero-Case-Study/"),
        summary_prefix: Some(
            "<a href=\"https://news.ycombinator.com/item?id=34858307\">Comments</a>",
        ),
        ..Case::DEFAULT
    },
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
    },
    Case {
        fixture: "feed-test-thumb.xml",
        title: Some("HDblog.it"),
        site_url: Some("https://www.hdblog.it"),
        description: Some("HDblog.it"),
        entry_id: Some("http://www.hdblog.it/?p=573063"),
        entry_title: Some(
            "Fairphone 5 è ufficiale: equo, modulare e aggiornato fino al 2031 | Anteprima IFA",
        ),
        entry_author: Some("HDblog.it"),
        entry_url: Some(
            "https://www.hdblog.it/smartphone/articoli/n573063/fairphone-5-ufficiale-caratteristiche-prezzo/",
        ),
        categories: Some(&["Android", "Fairphone"]),
        summary_prefix: Some(
            "Due fotocamere da 50MP sul retro, 5 anni di garanzia e 10 moduli sostituibili.",
        ),
        content_prefix: Some(
            "<a href=\"https://hd2.tudocdn.net/1113460?w=660\"><img src=\"https://hd2.tudocdn.ne",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-greek.xml",
        title: Some("Liberal - ΕΠΙΚΑΙΡΟΤΗΤΑ"),
        site_url: Some("https://www.liberal.gr/news"),
        description: Some(
            "Ενημέρωση με άποψη για την οικονομία, την πολιτική και τις διεθνείς σχέσεις.",
        ),
        entry_id: Some(
            "https://www.liberal.gr/news/portraita%2Dkarantinas%2Dgia%2Dempneusi%2Dapo%2Dti%2Dsullogi%2Dfeliou/298923",
        ),
        entry_title: Some("«Πορτραίτα καραντίνας» για έμπνευση από τη συλλογή Φέλιου"),
        entry_url: Some(
            "https://www.liberal.gr/news/portraita%2Dkarantinas%2Dgia%2Dempneusi%2Dapo%2Dti%2Dsullogi%2Dfeliou/298923",
        ),
        summary_prefix: Some(
            "Η συνθήκη του αναγκαστικού εγκλεισμού οδήγησε τα μεγάλα μουσεία διεθνώς να βρουν",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-hindi-channel-image.xml",
        title: Some("Latest News चीन News18 हिंदी"),
        site_url: Some("https://hindi.news18.com/rss/khabar/world/china.xml"),
        description: Some("Latest news from चीन Section"),
        entry_id: Some(
            "https://hindi.news18.com/news/world/who-experts-in-wuhan-probing-coronavirus-origins-meet-chinese-scientists-nodtg-3437941.html",
        ),
        entry_title: Some("कैसे फैला कोरोना वायरस, WHO की टीम करेगी चीनी अधिकारियों के साथ बैठक"),
        entry_url: Some(
            "https://hindi.news18.com/news/world/who-experts-in-wuhan-probing-coronavirus-origins-meet-chinese-scientists-nodtg-3437941.html",
        ),
        categories: Some(&[]),
        summary_prefix: Some(
            "<img src='https://images.news18.com/ibnkhabar/uploads/2021/01/who-team-1.jpg' he",
        ),
        ..Case::DEFAULT
    },
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
    },
    Case {
        fixture: "feed-test-image-channel-reverse.xml",
        title: Some("The Joe Rogan Experience"),
        site_url: Some("https://www.joerogan.com"),
        description: Some("The podcast of Comedian Joe Rogan.."),
        entry_id: Some("0d7147a3-f1c1-4ae6-bbf8-2e0a493639ca"),
        entry_title: Some("#1405 - Sober October 3 Recap"),
        entry_url: Some("http://traffic.libsyn.com/joeroganexp/p1405.mp3"),
        summary_prefix: Some(
            "Joe is joined by Ari Shaffir, Bert Kreischer & Tom Segura to recap their 3rd ann",
        ),
        content_prefix: Some(
            "Joe is joined by Ari Shaffir, Bert Kreischer & Tom Segura to recap their 3rd ann",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-image-empty-tag.xml",
        title: Some("Hacker Noon"),
        site_url: Some("https://hackernoon.com"),
        description: Some("How hackers start their afternoons."),
        entry_id: Some("https://hackernoon.com/miscellaneous-directions?source=rss"),
        entry_title: Some("MISCELLANEOUS DIRECTIONS."),
        entry_author: Some("Catharine Esther Beecher"),
        entry_url: Some("https://hackernoon.com/miscellaneous-directions?source=rss"),
        categories: Some(&["domestic-manuals"]),
        summary_prefix: Some(
            "Every woman should know how to direct in regard to the proper care of domestic a",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-image-enclosure.xml",
        title: Some("Centrum dopravního výzkumu, v. v. i. (RSS 2.0)"),
        site_url: Some("https://www.cdv.cz/"),
        description: Some(
            "Informace o novinkách na webu Centra dopravního výzkumu, pořádaných akcích v oblasti dopravy a odborných článcích z oblasti dopravy.",
        ),
        entry_title: Some("Podpora udržitelné aktivní mobility a nástroje práce s veřejností"),
        entry_url: Some(
            "https://www.cdv.cz/podpora-udrzitelne-aktivni-mobility-a-nastroje-prace-s-verejnosti/",
        ),
        categories: Some(&["event"]),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-image.xml",
        title: Some("Movie Reviews"),
        site_url: Some("https://movieweb.com/movie-reviews/"),
        description: Some("Movie Reviews at MovieWeb"),
        entry_id: Some("https://movieweb.com/the-sun-is-also-a-star-review/"),
        entry_title: Some(
            "The Sun Is Also a Star Review: Yara Shahidi & Charles Melton Elevate Teen Romance",
        ),
        entry_url: Some("https://movieweb.com/the-sun-is-also-a-star-review/"),
        summary_prefix: Some(
            "The Sun Is Also a Star is a diverse romance that bucks Hollywood's YA genre.",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-image-link.xml",
        title: Some("Bleacher Report - Front Page"),
        site_url: Some("https://bleacherreport.com"),
        description: Some(
            "Bleacher Report - The latest articles about sports news, rumors, teams, events, and more.",
        ),
        entry_id: Some(
            "https://bleacherreport.com/articles/10077424-orange-cassidy-retains-aew-international-title-at-double-or-nothing-2023",
        ),
        entry_title: Some(
            "Orange Cassidy Retains AEW International Title at Double or Nothing 2023",
        ),
        entry_url: Some(
            "https://bleacherreport.com/articles/10077424-orange-cassidy-retains-aew-international-title-at-double-or-nothing-2023",
        ),
        categories: Some(&[
            "Breaking News",
            "All Elite Wrestling",
            "BNT MISC",
            "Orange Cassidy",
        ]),
        summary_prefix: Some(
            "Orange Cassidy won a 21-man Blackjack Battle Royal at Double or Nothing on Sunda",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-item-author.xml",
        title: Some("Planet KDE | English"),
        site_url: Some("https://planet.kde.org/"),
        description: Some("Planet KDE | English"),
        entry_id: Some("https://blogs.kde.org/2025/03/24/this-week-in-kde-apps/"),
        entry_title: Some("This Week in KDE Apps"),
        entry_author: Some("This Week in KDE Apps"),
        entry_url: Some("https://blogs.kde.org/2025/03/24/this-week-in-kde-apps/"),
        summary_prefix: Some(
            "<h4>Stability improvements in KDE PIM-land, and polls in NeoChat</h4>",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-item-channel-image.xml",
        title: Some("www.espn.com - TOP"),
        site_url: Some("https://www.espn.com"),
        description: Some("Latest TOP news from www.espn.com"),
        entry_id: Some("31393791"),
        entry_title: Some("Inside the mysterious world of missing sports memorabilia"),
        entry_url: Some(
            "https://www.espn.com/mlb/story/_/id/31393791/inside-mysterious-world-missing-sports-memorabilia",
        ),
        summary_prefix: Some(
            "Some of the most treasured pieces of sports memorabilia are missing, can't be au",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-itunes.xml",
        title: Some("The Joe Rogan Experience"),
        site_url: Some("https://www.joerogan.com"),
        description: Some("Conduit to the Gaian Mind"),
        entry_id: Some("00a5d989b6b2cd8267cf8239f3b5585c"),
        entry_title: Some("#1109 - Matthew Walker"),
        entry_url: Some("http://traffic.libsyn.com/joeroganexp/p1109.mp3"),
        categories: Some(&[]),
        summary_prefix: Some(
            "Matthew Walker is Professor of Neuroscience and Psychology at the University of ",
        ),
        content_prefix: Some(
            "Matthew Walker is Professor of Neuroscience and Psychology at the University of ",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-itunes-season.xml",
        title: Some("With Gourley And Rust"),
        site_url: Some("https://www.patreon.com/withgourleyandrust"),
        description: Some(
            "Nothing says slasher films and horror franchises more than an easy-listening, cozy-cast by two gentle movie lovers like <a href=\"mattgourley.com\" rel=\"noopener noreferrer\" target=\"_blank\">Matt Gourley</a> and <a href=\"twitter.com/paulrust\" rel=\"noopener noreferrer\" target=\"_blank\">Paul Rust</a>. Join them as they take you through the Jasons, the Michaels, the Freddies, and beyond. Get past seasons such as IN MYERS and KRUEGER WE TRUST, lots of bonus content, and regular episodes a week early at patreon.com/withgourleyandrust.<br /><hr><p style='color:grey; font-size:0.75em;'> Hosted on Acast. See <a style='color:grey;' target='_blank' rel='noopener noreferrer' href='https://acast.com/privacy'>acast.com/privacy</a> for more information.</p>",
        ),
        entry_id: Some("635861df5f0a000012334a6f"),
        entry_title: Some("RASING CAIN"),
        entry_url: Some("https://www.patreon.com/withgourleyandrust"),
        summary_prefix: Some(
            "<p>The De Palma Paradox.</p><br><p>With Gourley And Rust bonus content on <a hre",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-multiple-image-and-video.xml",
        title: Some("Motor.ru"),
        site_url: Some("https://motor.ru"),
        description: Some("Автомобильное издание Lenta.ru"),
        entry_title: Some("Kia сомневается в перспективах Stinger второго поколения"),
        entry_author: Some("Александр Пономарев"),
        entry_url: Some("https://motor.ru/news/kia-stinger-future-25-04-2020.htm"),
        categories: Some(&["Новости"]),
        summary_prefix: Some(
            "Планируемый рестайлинг Kia Stinger окажется весьма масштабным, но появление втор",
        ),
        content_prefix: Some(
            "<p>Планируемый рестайлинг Kia Stinger окажется весьма масштабным, но появление в",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-multiple-media-content.xml",
        title: Some("Nuclearpasta"),
        site_url: Some("https://mastodon.art/@nuclearpasta"),
        description: Some("Public posts from @nuclearpasta@mastodon.art"),
        entry_id: Some("https://mastodon.art/@nuclearpasta/114495761108890085"),
        entry_url: Some("https://mastodon.art/@nuclearpasta/114495761108890085"),
        categories: Some(&["webcomicday", "webcomic", "fantasy", "comics"]),
        summary_prefix: Some(
            "<p>Oh it&#39;s <a href=\"https://mastodon.art/tags/WebcomicDay\" class=\"mention ha",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-source.xml",
        title: Some("À la une - Google Actualités"),
        site_url: Some("https://news.google.com/?hl=fr&gl=BE&ceid=BE:fr"),
        description: Some("Google Actualités"),
        entry_id: Some("52782335008021"),
        entry_title: Some(
            "Coronavirus en Belgique: 200 cas détectés au total, un nombre limité de tests effectués par manque de réactifs - RTBF",
        ),
        entry_url: Some(
            "https://news.google.com/__i/rss/rd/articles/CBMiiwFodHRwczovL3d3dy5ydGJmLmJlL2luZm8vc29jaWV0ZS9kZXRhaWxfY29yb25hdmlydXMtZW4tYmVsZ2lxdWUtMjAwLWNhcy1kZXRlY3Rlcy1hdS10b3RhbC1sYS1jaXJjdWxhdGlvbi1kdS12aXJ1cy1yZXN0ZS1saW1pdGVlP2lkPTEwNDUwNzkz0gEA?oc=5",
        ),
        summary_prefix: Some(
            "<ol><li><a href=\"https://news.google.com/__i/rss/rd/articles/CBMiiwFodHRwczovL3d",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-spaced-link.xml",
        title: Some("Sixth Tone RSS"),
        site_url: Some("https://www.sixthtone.com"),
        description: Some("Sixth Tone RSS"),
        entry_title: Some("Feathered Dinosaur From China Points to Another Path to Flight"),
        entry_author: Some("Fan Yiying"),
        entry_url: Some(
            "https://www.sixthtone.com/news/1019094/Feathered Dinosaur From China Points to Another Path to Flight",
        ),
        categories: Some(&["NEWS"]),
        summary_prefix: Some(
            "<a href=\"https://www.sixthtone.com/news/1019094/Feathered Dinosaur From China Po",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test.xml",
        title: Some("Android Authority"),
        site_url: Some("https://www.androidauthority.com"),
        description: Some("Android News, Reviews, How To"),
        entry_id: Some("https://www.androidauthority.com/?p=986823"),
        entry_title: Some("Japan’s plans for 5G include 10 billion 14-digit phone numbers"),
        entry_author: Some("Williams Pelegrin"),
        entry_url: Some(
            "https://www.androidauthority.com/japan-phone-numbers-14-digits-5g-986823/",
        ),
        categories: Some(&["News", "5G", "Japan"]),
        summary_prefix: Some(
            "Japan expects a huge surge of 5G-enabled devices to hit the market in the future",
        ),
        content_prefix: Some(
            "<p><a href=\"https://cdn57.androidauthority.net/wp-content/uploads/2019/02/Whats-",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-time.xml",
        title: Some("Drug Recalls"),
        site_url: Some(
            "http://www.fda.gov/about-fda/contact-fda/stay-informed/rss-feeds/drug-recalls/rss.xml",
        ),
        entry_id: Some(
            "http://www.fda.gov/safety/recalls-market-withdrawals-safety-alerts/vivimed-life-sciences-pvt-ltd-issues-voluntary-nationwide-recall-losartan-potassium-25-mg-50-mg-and",
        ),
        entry_title: Some(
            "Vivimed Life Sciences Pvt Ltd Issues Voluntary Nationwide Recall of Losartan\n                Potassium 25 mg, 50 mg and 100 mg Tablets, USP Due to the Detection of Trace Amounts\n                of N-Nitroso-N-methyl-4-aminobutyric acid (NMBA) Impurity",
        ),
        entry_author: Some("FDA"),
        entry_url: Some(
            "http://www.fda.gov/safety/recalls-market-withdrawals-safety-alerts/vivimed-life-sciences-pvt-ltd-issues-voluntary-nationwide-recall-losartan-potassium-25-mg-50-mg-and",
        ),
        summary_prefix: Some(
            "Vivimed Life Sciences Pvt Ltd (Vivimed) is recalling 19 lots of Losartan",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-unexpected-token.xml",
        title: Some("Wheels Off-Road & 4x4"),
        site_url: Some("https://www.wheels24.co.za/"),
        entry_title: Some(
            "Wheels24.co.za | WATCH | Range Rover spices up its Velar line-up, adds new hybrid model to the range",
        ),
        entry_url: Some(
            "https://www.wheels24.co.za/OffRoad_and_4x4/Bakkie_and_SUV/watch-range-rover-spices-up-its-velar-line-up-adds-new-hybrid-model-to-the-range-20200923-3",
        ),
        summary_prefix: Some(
            "Range Rover updated its desirable Velar range by adding new engine choices and a",
        ),
        ..Case::DEFAULT
    },
    Case {
        fixture: "feed-test-xsl.xml",
        title: Some("SkySports | Liverpool"),
        site_url: Some("http://www.skysports.com"),
        description: Some("Liverpool News"),
        entry_title: Some("Insight: Who should Liverpool sign?"),
        entry_url: Some(
            "https://www.skysports.com/football/news/11669/11719097/premier-league-transfer-window-who-should-liverpool-sign",
        ),
        categories: Some(&["News Story"]),
        summary_prefix: Some(
            "Liverpool just missed out on clinching the Premier League title and have a Champ",
        ),
        ..Case::DEFAULT
    },
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
    },
];

/// Media these fixtures were written to test: YouTube's media group,
/// `media:content` images, and audio or video enclosures.
#[test]
fn reads_media() {
    let youtube = first_entry("atom-feed-youtube.xml");
    let media = youtube.media.unwrap();
    assert_eq!(
        youtube.youtube.unwrap().video_id.as_deref(),
        Some("W_S7iKZZdHw")
    );
    assert_eq!(
        media.title.as_deref(),
        Some("Digital Foundry Games of 2024: The Oliver Mackenzie Collection!")
    );
    assert_eq!(
        media.contents[0].url,
        "https://www.youtube.com/v/W_S7iKZZdHw?version=3"
    );
    assert_eq!(
        media.thumbnails[0].url,
        "https://i4.ytimg.com/vi/W_S7iKZZdHw/hqdefault.jpg"
    );
    assert!(media.description.unwrap().starts_with(
        "As the holiday period begins to unwind, we continue to look back on an excellent 2024"
    ));

    let mastodon = first_entry("feed-multiple-media-content.xml")
        .media
        .unwrap();
    let contents: Vec<_> = mastodon
        .contents
        .iter()
        .map(|content| (content.url.as_str(), content.mime_type.as_deref()))
        .collect();
    let files = "https://cdn.masto.host/mastodonart/media_attachments/files/114/495";
    assert_eq!(
        contents,
        [
            (
                &*format!("{files}/754/804/385/848/original/dee105af83a73921.jpg"),
                Some("image/jpeg")
            ),
            (
                &*format!("{files}/755/611/292/322/original/c332f57157abc203.jpg"),
                Some("image/jpeg")
            ),
            (
                &*format!("{files}/756/453/333/085/original/f0baf8d5c4be0461.jpg"),
                Some("image/jpeg")
            ),
            (
                &*format!("{files}/757/644/563/056/original/884111c48e491831.jpg"),
                Some("image/jpeg")
            ),
        ]
    );
    assert_eq!(
        mastodon.contents[0].description.as_deref(),
        Some("Raiders ride giant lizards across the desert.")
    );

    let audio = [
        (
            "feed-itunes-season.xml",
            "https://sphinx.acast.com/p/acast/s/with-gourley-and-rust/e/635861df5f0a000012334a6f/media.mp3",
        ),
        (
            "feed-test-charset.xml",
            "https://podkast.nrk.no/fil/loerdagsraadet/23ee45fc-d869-4110-bf97-34138fcfaba6_0_ID192MP3.mp3",
        ),
        (
            "feed-test-image-channel-reverse.xml",
            "http://traffic.libsyn.com/joeroganexp/p1405.mp3?dest-id=19997",
        ),
    ];
    for (fixture, url) in audio {
        let entry = first_entry(fixture);
        assert_eq!(
            entry.enclosure.map(|e| e.url).as_deref(),
            Some(url),
            "{fixture}"
        );
        assert_eq!(entry.enclosures[0].url, url, "{fixture}");
    }

    let video = first_entry("feed-test-multiple-image-and-video.xml");
    let video = video
        .enclosures
        .iter()
        .find(|enclosure| enclosure.mime_type.as_deref() == Some("video/mp4"))
        .map(|enclosure| enclosure.url.as_str());
    assert_eq!(
        video,
        Some("https://motor.media.eagleplatform.com/index/player?record_id=1410861")
    );
}

fn first_entry(fixture: &str) -> mariner_feeds::Entry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rssparser");
    let bytes = std::fs::read(format!("{path}/{fixture}")).unwrap();
    mariner_feeds::parse(&bytes).unwrap().entries.remove(0)
}
