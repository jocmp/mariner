//! `feed-test-time.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
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
    }
    .check();
}
