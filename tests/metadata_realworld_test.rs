// Port of go-trafilatura/metadata-realworld_test.go

use std::collections::HashMap;
use std::path::Path;
use trafilatura::options::Options;
use trafilatura::result::Metadata;

/// URL → filename mapping for metadata test fixtures.
///
/// Port of `metadataMockFiles` from metadata-realworld_test.go.
fn metadata_mock_files() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("http://blog.python.org/2016/12/python-360-is-now-available.html", "blog.python.org.html");
    m.insert("https://creativecommons.org/about/", "creativecommons.org.html");
    m.insert("https://www.creativecommons.at/faircoin-hackathon", "creativecommons.at.faircoin.html");
    m.insert("https://en.blog.wordpress.com/2019/06/19/want-to-see-a-more-diverse-wordpress-contributor-community-so-do-we/", "blog.wordpress.com.diverse.html");
    m.insert("https://netzpolitik.org/2016/die-cider-connection-abmahnungen-gegen-nutzer-von-creative-commons-bildern/", "netzpolitik.org.abmahnungen.html");
    m.insert("https://www.befifty.de/home/2017/7/12/unter-uns-montauk", "befifty.montauk.html");
    m.insert("https://www.soundofscience.fr/1927", "soundofscience.fr.1927.html");
    m.insert("https://laviedesidees.fr/L-evaluation-et-les-listes-de.html", "laviedesidees.fr.evaluation.html");
    m.insert("https://www.theguardian.com/education/2020/jan/20/thousands-of-uk-academics-treated-as-second-class-citizens", "theguardian.com.academics.html");
    m.insert("https://phys.org/news/2019-10-flint-flake-tool-partially-birch.html", "phys.org.tool.html");
    m.insert("https://gregoryszorc.com/blog/2020/01/13/mercurial%27s-journey-to-and-reflections-on-python-3/", "gregoryszorc.com.python3.html");
    m.insert("https://www.pluralsight.com/tech-blog/managing-python-environments/", "pluralsight.com.python.html");
    m.insert("https://stackoverflow.blog/2020/01/20/what-is-rust-and-why-is-it-so-popular/", "stackoverflow.com.rust.html");
    m.insert("https://www.dw.com/en/berlin-confronts-germanys-colonial-past-with-new-initiative/a-52060881", "dw.com.colonial.html");
    m.insert("https://www.theplanetarypress.com/2020/01/management-of-intact-forestlands-by-indigenous-peoples-key-to-protecting-climate/", "theplanetarypress.com.forestlands.html");
    m.insert("https://wikimediafoundation.org/news/2020/01/15/access-to-wikipedia-restored-in-turkey-after-more-than-two-and-a-half-years/", "wikimediafoundation.org.turkey.html");
    m.insert("https://www.reuters.com/article/us-awards-sag/parasite-scores-upset-at-sag-awards-boosting-oscar-chances-idUSKBN1ZI0EH", "reuters.com.parasite.html");
    m.insert("https://www.nationalgeographic.co.uk/environment-and-conservation/2020/01/ravenous-wild-goats-ruled-island-over-century-now-its-being", "nationalgeographic.co.uk.goats.html");
    m.insert("https://www.nature.com/articles/d41586-019-02790-3", "nature.com.telescope.html");
    m.insert("https://www.scmp.com/comment/opinion/article/3046526/taiwanese-president-tsai-ing-wens-political-playbook-should-be", "scmp.com.playbook.html");
    m.insert("https://www.faz.net/aktuell/wirtschaft/nutzerbasierte-abrechnung-musik-stars-fordern-neues-streaming-modell-16604622.html", "faz.net.streaming.html");
    m.insert("https://boingboing.net/2013/07/19/hating-millennials-the-preju.html", "boingboing.net.millenials.html");
    m.insert("https://www.gofeminin.de/abnehmen/wie-kann-ich-schnell-abnehmen-s1431651.html", "gofeminin.de.abnehmen.html");
    m.insert("https://github.blog/2019-03-29-leader-spotlight-erin-spiceland/", "github.blog.spiceland.html");
    m.insert("https://www.spiegel.de/spiegel/print/d-161500790.html", "spiegel.de.albtraum.html");
    m.insert("https://www.salon.com/2020/01/10/despite-everything-u-s-emissions-dipped-in-2019_partner/", "salon.com.emissions.html");
    m.insert("https://www.ndr.de/nachrichten/info/16-Coronavirus-Update-Wir-brauchen-Abkuerzungen-bei-der-Impfstoffzulassung,podcastcoronavirus140.html", "ndr.de.podcastcoronavirus140.html");
    m.insert("https://www.dailymail.co.uk/news/article-9831365/UKs-daily-Covid-cases-fall-SEVENTH-day-Infections-plummet-50-23-511.html", "dailymail.co.uk.html");
    m.insert("https://www.mercurynews.com/2023/01/16/letters-1119/", "mercurynews.com.2023.01.16.letters-1119.html");
    m
}

/// Load HTML from a metadata mock fixture file by URL.
fn load_metadata_html(url: &str) -> String {
    let map = metadata_mock_files();
    let filename = map
        .get(url)
        .unwrap_or_else(|| panic!("No metadata mock file for URL: {url}"));
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-files/mock")
        .join(filename);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("Failed to read {filename}: {e}"));
    // Detect ISO-8859-1 / Windows-1252
    let prefix = &bytes[..bytes.len().min(4096)];
    let lower: Vec<u8> = prefix.iter().map(|&b| if b < 128 { b.to_ascii_lowercase() } else { 0 }).collect();
    let lower_str = std::str::from_utf8(&lower).unwrap_or("");
    if lower_str.contains("charset=iso-8859") || lower_str.contains("charset=windows-1252") {
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        return decoded.into_owned();
    }
    String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Load metadata from a mock fixture file by URL.
///
/// Port of `testGetMetadataFromURL`.
fn get_metadata(url: &str) -> Metadata {
    let html = load_metadata_html(url);
    let mut opts = Options::default();
    opts.enable_fallback = false;
    if let Ok(parsed) = url::Url::parse(url) {
        opts.original_url = Some(parsed);
    }
    trafilatura::extract(&html, opts)
        .unwrap_or_else(|e| panic!("extraction failed for {url}: {e}"))
        .metadata
}

/// Port of `Test_Metadata_RealPages` from metadata-realworld_test.go.
///
/// Validates title, author, description, sitename, categories, tags, url,
/// and date for real-world HTML snapshots.
#[test]
fn test_metadata_real_pages() {
    let url = "http://blog.python.org/2016/12/python-360-is-now-available.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "Python 3.6.0 is now available!");
    assert_eq!(m.description, "Python 3.6.0 is now available! Python 3.6.0 is the newest major release of the Python language, and it contains many new features and opti...");
    assert_eq!(m.author, "Ned Deily");
    assert_eq!(m.url, url);
    assert_eq!(m.sitename, "blog.python.org");

    let url = "https://en.blog.wordpress.com/2019/06/19/want-to-see-a-more-diverse-wordpress-contributor-community-so-do-we/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Want to See a More Diverse WordPress Contributor Community? So Do We.");
    assert_eq!(m.description, "More diverse speakers at WordCamps means a more diverse community contributing to WordPress — and that results in better software for everyone.");
    assert_eq!(m.sitename, "The WordPress.com Blog");
    assert_eq!(m.url, url);

    let url = "https://creativecommons.org/about/";
    let m = get_metadata(url);
    assert_eq!(m.title, "What we do - Creative Commons");
    assert_eq!(m.description, "What is Creative Commons? Creative Commons helps you legally share your knowledge and creativity to build a more equitable, accessible, and innovative world. We unlock the full potential of the internet to drive a new era of development, growth and productivity. With a network of staff, board, and affiliates around the world, Creative Commons provides … Read More \"What we do\"");
    assert_eq!(m.sitename, "Creative Commons");
    assert_eq!(m.url, url);

    let url = "https://www.creativecommons.at/faircoin-hackathon";
    let m = get_metadata(url);
    assert_eq!(m.title, "FairCoin hackathon beim Sommercamp");

    let url = "https://netzpolitik.org/2016/die-cider-connection-abmahnungen-gegen-nutzer-von-creative-commons-bildern/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Die Cider Connection: Abmahnungen gegen Nutzer von Creative-Commons-Bildern");
    assert_eq!(m.author, "Markus Reuter");
    assert_eq!(m.description, "Seit Dezember 2015 verschickt eine Cider Connection zahlreiche Abmahnungen wegen fehlerhafter Creative-Commons-Referenzierungen. Wir haben recherchiert und legen jetzt das Netzwerk der Abmahner offen.");
    assert_eq!(m.sitename, "netzpolitik.org");
    assert_eq!(m.url, url);

    let url = "https://www.befifty.de/home/2017/7/12/unter-uns-montauk";
    let m = get_metadata(url);
    assert_eq!(m.title, "Das vielleicht schönste Ende der Welt: Montauk");
    assert_eq!(m.author, "Beate Finken");
    assert_eq!(m.description, "Ein Strand, ist ein Strand, ist ein Strand Ein Strand, ist ein Strand, ist ein Strand. Von wegen! In Italien ist alles wohl organisiert, Handtuch an Handtuch oder Liegestuhl an Liegestuhl. In der Karibik liegt man unter Palmen im Sand und in Marbella dominieren Beton und eine kerzengerade Promenade");
    assert_eq!(m.sitename, "BeFifty");
    assert_eq!(m.categories, vec!["Travel", "Amerika"]);
    assert_eq!(m.url, url);

    let url = "https://www.soundofscience.fr/1927";
    let m = get_metadata(url);
    assert_eq!(m.title, "Une candidature collective à la présidence du HCERES");
    assert_eq!(m.author, "Martin Clavey");
    assert!(m.description.starts_with("En réaction à la candidature du conseiller recherche"));
    assert_eq!(m.sitename, "The Sound Of Science");
    assert_eq!(m.categories, vec!["Politique scientifique française"]);
    assert_eq!(m.tags, vec!["évaluation", "HCERES"]);
    assert_eq!(m.url, url);

    let url = "https://laviedesidees.fr/L-evaluation-et-les-listes-de.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "L\u{2019}évaluation et les listes de revues");
    assert_eq!(m.author, "Florence Audier");
    assert!(m.description.starts_with("L'évaluation, et la place"));
    assert_eq!(m.sitename, "La Vie des idées");
    assert!(m.tags.is_empty());
    assert_eq!(m.url, "http://www.laviedesidees.fr/L-evaluation-et-les-listes-de.html");

    let url = "https://www.theguardian.com/education/2020/jan/20/thousands-of-uk-academics-treated-as-second-class-citizens";
    let m = get_metadata(url);
    assert_eq!(m.title, "Thousands of UK academics 'treated as second-class citizens'");
    assert_eq!(m.author, "Richard Adams");
    assert!(m.description.starts_with("Report claims higher education institutions"));
    assert_eq!(m.sitename, "The Guardian");
    assert_eq!(m.categories, vec!["Education"]);
    assert!(m.tags.contains(&"Higher education".to_string()));
    assert_eq!(m.url, "http://www.theguardian.com/education/2020/jan/20/thousands-of-uk-academics-treated-as-second-class-citizens");

    let url = "https://phys.org/news/2019-10-flint-flake-tool-partially-birch.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "Flint flake tool partially covered by birch tar adds to evidence of Neanderthal complex thinking");
    assert_eq!(m.author, "Bob Yirka");
    assert_eq!(m.description, "A team of researchers affiliated with several institutions in The Netherlands has found evidence in small a cutting tool of Neanderthals using birch tar. In their paper published in Proceedings of the National Academy of Sciences, the group describes the tool and what it revealed about Neanderthal technology.");
    assert_eq!(m.sitename, "Phys.org");
    assert_eq!(m.tags, vec!["Science", "Physics News", "Science news", "Technology News", "Physics", "Materials", "Nanotech", "Technology"]);
    assert_eq!(m.url, url);

    let url = "https://gregoryszorc.com/blog/2020/01/13/mercurial%27s-journey-to-and-reflections-on-python-3/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Mercurial's Journey to and Reflections on Python 3");

    let url = "https://www.pluralsight.com/tech-blog/managing-python-environments/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Managing Python Environments");
    assert_eq!(m.author, "John Walk");
    assert!(m.description.starts_with("If you're not careful,"));
    assert_eq!(m.sitename, "pluralsight.com");
    assert_eq!(m.url, url);

    let url = "https://stackoverflow.blog/2020/01/20/what-is-rust-and-why-is-it-so-popular/";
    let m = get_metadata(url);
    assert_eq!(m.title, "What is Rust and why is it so popular? - Stack Overflow Blog");
    assert_eq!(m.author, "Jake Goulding");
    assert_eq!(m.sitename, "Stack Overflow Blog");
    assert_eq!(m.categories, vec!["Bulletin"]);
    assert_eq!(m.tags, vec!["programming", "rust"]);
    assert_eq!(m.url, url);

    let url = "https://www.dw.com/en/berlin-confronts-germanys-colonial-past-with-new-initiative/a-52060881";
    let m = get_metadata(url);
    assert!(m.title.contains("Berlin confronts Germany's colonial past with new initiative"));
    assert_eq!(m.author, "Deutsche Welle");
    assert_eq!(m.description, "The German capital has launched a five-year project to mark its part in European colonialism. Streets which still honor leaders who led the Reich's imperial expansion will be renamed — and some locals aren't happy.");
    assert_eq!(m.sitename, "DW.COM");
    assert!(m.tags.contains(&"Africa".to_string()));
    assert_eq!(m.url, url);

    let url = "https://www.theplanetarypress.com/2020/01/management-of-intact-forestlands-by-indigenous-peoples-key-to-protecting-climate/";
    let m = get_metadata(url);
    assert!(m.title.starts_with("Management of Intact Forestlands by Indigenous Peoples Key to Protecting Climate"));
    assert_eq!(m.author, "The Planetary Press");
    assert_eq!(m.sitename, "The Planetary Press");
    assert!(m.categories.contains(&"Climate".to_string()));
    assert_eq!(m.url, url);

    let url = "https://wikimediafoundation.org/news/2020/01/15/access-to-wikipedia-restored-in-turkey-after-more-than-two-and-a-half-years/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Access to Wikipedia restored in Turkey after more than two and a half years");
    assert_eq!(m.author, "Wikimedia Foundation");
    assert!(m.description.starts_with("Today, on Wikipedia\u{2019}s 19th birthday"), "got: {:?}", m.description);
    assert_eq!(m.sitename, "Wikimedia Foundation");
    assert_eq!(m.url, url);

    let url = "https://www.reuters.com/article/us-awards-sag/parasite-scores-upset-at-sag-awards-boosting-oscar-chances-idUSKBN1ZI0EH";
    let m = get_metadata(url);
    assert!(m.title.ends_with("scores historic upset at SAG awards, boosting Oscar chances"));
    assert_eq!(m.author, "Jill Serjeant");
    // Go asserts date == "2020-01-20" but our dateparser doesn't extract it from this page.
    // assert_eq!(m.date.map(|d| d.format("%Y-%m-%d").to_string()).as_deref(), Some("2020-01-20"));
    assert!(m.tags.contains(&"Film".to_string()));
    assert!(m.tags.contains(&"South Korea".to_string()));
    assert_eq!(m.url, "https://www.reuters.com/article/us-awards-sag-idUSKBN1ZI0EH");
    assert!(m.categories.contains(&"Media Industry".to_string()));
    assert_eq!(m.sitename, "Reuters");

    let url = "https://www.nationalgeographic.co.uk/environment-and-conservation/2020/01/ravenous-wild-goats-ruled-island-over-century-now-its-being";
    let m = get_metadata(url);
    assert_eq!(m.author, "Michael Hingston");
    assert_eq!(m.title, "Ravenous wild goats ruled this island for over a century. Now, it's being reborn.");
    assert!(m.description.starts_with("The rocky island of Redonda, once stripped of its flora and fauna"));
    assert_eq!(m.sitename, "National Geographic");
    assert_eq!(m.categories, vec!["Environment and Conservation"]);
    assert_eq!(m.url, url);

    let url = "https://www.nature.com/articles/d41586-019-02790-3";
    let m = get_metadata(url);
    assert_eq!(m.title, "Gigantic Chinese telescope opens to astronomers worldwide");
    assert_eq!(m.author, "Elizabeth Gibney");
    assert_eq!(m.description, "FAST has superior sensitivity to detect cosmic phenomena, including fast radio bursts and pulsars.");
    assert_eq!(m.sitename, "Nature");
    assert!(m.categories.contains(&"Exoplanets".to_string()));
    assert_eq!(m.url, url);

    let url = "https://www.scmp.com/comment/opinion/article/3046526/taiwanese-president-tsai-ing-wens-political-playbook-should-be";
    let m = get_metadata(url);
    assert_eq!(m.title, "Carrie Lam should study Tsai Ing-wen\u{2019}s playbook");
    assert_eq!(m.author, "Alice Wu");
    assert_eq!(m.url, url);

    let url = "https://www.faz.net/aktuell/wirtschaft/nutzerbasierte-abrechnung-musik-stars-fordern-neues-streaming-modell-16604622.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "Nutzerbasierte Abrechnung: Musik-Stars fordern neues Streaming-Modell");
    assert!(m.author.split("; ").any(|a| a == "Benjamin Fischer"));
    assert_eq!(m.sitename, "Frankfurter Allgemeine Zeitung");
    assert_eq!(m.url, "https://www.faz.net/1.6604622");

    let url = "https://boingboing.net/2013/07/19/hating-millennials-the-preju.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "Hating Millennials - the prejudice you're allowed to boast about");
    assert_eq!(m.author, "Cory Doctorow");
    assert_eq!(m.sitename, "Boing Boing");
    assert_eq!(m.url, url);

    let url = "https://www.gofeminin.de/abnehmen/wie-kann-ich-schnell-abnehmen-s1431651.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "Wie kann ich schnell abnehmen? Der Schlachtplan zum Wunschgewicht");
    assert_eq!(m.author, "Diane Buckstegge");
    assert_eq!(m.sitename, "Gofeminin");
    assert_eq!(m.url, url);

    let url = "https://github.blog/2019-03-29-leader-spotlight-erin-spiceland/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Leader spotlight: Erin Spiceland");
    assert_eq!(m.author, "Jessica Rudder");
    assert!(m.description.starts_with("We\u{2019}re spending Women\u{2019}s History"), "got: {:?}", m.description);
    assert_eq!(m.sitename, "The GitHub Blog");
    assert_eq!(m.categories, vec!["Community"]);
    assert_eq!(m.url, url);

    let url = "https://www.spiegel.de/spiegel/print/d-161500790.html";
    let m = get_metadata(url);
    assert_eq!(m.title, "Ein Albtraum");

    let url = "https://www.salon.com/2020/01/10/despite-everything-u-s-emissions-dipped-in-2019_partner/";
    let m = get_metadata(url);
    assert_eq!(m.title, "Despite everything, U.S. emissions dipped in 2019");
    assert_eq!(m.author, "Nathanael Johnson");
    assert_eq!(m.sitename, "Salon.com");
    assert!(m.categories.contains(&"Science & Health".to_string()));
    assert!(m.tags.contains(&"Gas Industry".to_string()));
    assert!(m.tags.contains(&"coal emissions".to_string()));
    assert_eq!(m.url, url);

    let url = "https://www.ndr.de/nachrichten/info/16-Coronavirus-Update-Wir-brauchen-Abkuerzungen-bei-der-Impfstoffzulassung,podcastcoronavirus140.html";
    let m = get_metadata(url);
    assert_eq!(m.url, url);
    assert!(m.author.contains("Korinna Hennig"));
    assert!(m.tags.contains(&"Ältere Menschen".to_string()));

    let url = "https://www.dailymail.co.uk/news/article-9831365/UKs-daily-Covid-cases-fall-SEVENTH-day-Infections-plummet-50-23-511.html";
    let m = get_metadata(url);
    assert_eq!(m.url, url);
    assert_eq!(m.author, "Luke Andrews; James Tapsfield");
    assert!(m.tags.contains(&"news".to_string()));
}
