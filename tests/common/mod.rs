// Test helpers — port of go-trafilatura/helper_test.go and realworld-mock_test.go

use std::collections::HashMap;
use std::path::Path;

use trafilatura::options::Options;
use trafilatura::result::ExtractResult;

/// Detect a charset declaration from the first part of an HTML byte sequence.
///
/// Looks for `charset=xxx` in the first 2KB. Since the charset declaration itself
/// is ASCII text, this works even for ISO-8859-1 files (ASCII bytes are the same
/// in both encodings).
fn detect_charset(bytes: &[u8]) -> Option<&'static str> {
    let prefix = &bytes[..bytes.len().min(4096)];
    // Map bytes to ASCII lowercase for comparison, treating non-ASCII bytes as 0.
    let lower: Vec<u8> = prefix
        .iter()
        .map(|&b| if b < 128 { b.to_ascii_lowercase() } else { 0 })
        .collect();
    let lower_str = std::str::from_utf8(&lower).unwrap_or("");
    if lower_str.contains("charset=iso-8859") || lower_str.contains("charset=\"iso-8859") {
        Some("iso-8859-1")
    } else if lower_str.contains("charset=windows-1252")
        || lower_str.contains("charset=\"windows-1252")
    {
        Some("windows-1252")
    } else {
        None
    }
}

/// Load mock HTML file content by URL.
///
/// Handles ISO-8859-1 / Latin-1 encoded files by decoding them to UTF-8
/// before passing to the extractor, matching how Go's html.Parse handles
/// charset meta tags.
///
/// Port of `openMockFile`.
pub fn load_mock_html(url: &str) -> String {
    let filename = mock_file_map()
        .get(url)
        .unwrap_or_else(|| panic!("No mock file registered for URL: {url}"))
        .to_string();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-files/mock")
        .join(&filename);
    // Read as bytes first to handle non-UTF-8 encodings (e.g. ISO-8859-1).
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read mock file {filename}: {e}"));

    // Try to detect and convert non-UTF-8 charsets before parsing.
    if detect_charset(&bytes).is_some() {
        // Use encoding_rs for proper charset conversion.
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        return decoded.into_owned();
    }

    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Extract content from a mock file, optionally including links.
///
/// Port of `extractMockFile`.
pub fn extract_mock_file(url: &str, include_links: bool) -> Option<ExtractResult> {
    let html = load_mock_html(url);
    let mut opts = Options::default();
    opts.enable_fallback = true;
    opts.include_links = include_links;
    if let Ok(parsed) = url::Url::parse(url) {
        opts.original_url = Some(parsed);
    }
    trafilatura::extract(&html, &opts).ok()
}

/// True when the text appears in content_text or comments_text.
pub fn res_contains(result: &ExtractResult, s: &str) -> bool {
    result.content_text.contains(s) || result.comments_text.contains(s)
}

/// True when the string appears in content_html or comments_html.
pub fn html_contains(result: &ExtractResult, s: &str) -> bool {
    result.content_html.contains(s) || result.comments_html.contains(s)
}

/// URL → mock filename mapping.
///
/// Port of `rwMockFiles` from `realworld-mock_test.go`.
pub fn mock_file_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("http://exotic_tags", "exotic_tags.html");
    m.insert(
        "https://die-partei.net/luebeck/2012/05/31/das-ministerium-fur-club-kultur-informiert/",
        "die-partei.net.luebeck.html",
    );
    m.insert(
        "https://www.bmjv.de/DE/Verbraucherportal/KonsumImAlltag/TransparenzPreisanpassung/TransparenzPreisanpassung_node.html",
        "bmjv.de.konsum.html",
    );
    m.insert(
        "http://kulinariaathome.wordpress.com/2012/12/08/mandelplatzchen/",
        "kulinariaathome.com.mandelplätzchen.html",
    );
    m.insert(
        "https://denkanstoos.wordpress.com/2012/04/11/denkanstoos-april-2012/",
        "denkanstoos.com.2012.html",
    );
    m.insert(
        "https://www.demokratiewebstatt.at/thema/thema-umwelt-und-klima/woher-kommt-die-dicke-luft",
        "demokratiewebstatt.at.luft.html",
    );
    m.insert(
        "http://www.toralin.de/schmierfett-reparierend-verschlei-y-910.html",
        "toralin.de.schmierfett.html",
    );
    m.insert(
        "https://www.ebrosia.de/beringer-zinfandel-rose-stone-cellars-lieblich-suess",
        "ebrosia.de.zinfandel.html",
    );
    m.insert(
        "https://www.landwirt.com/Precision-Farming-Moderne-Sensortechnik-im-Kuhstall,,4229,,Bericht.html",
        "landwirt.com.sensortechnik.html",
    );
    m.insert(
        "http://schleifen.ucoz.de/blog/briefe/2010-10-26-18",
        "schleifen.ucoz.de.briefe.html",
    );
    m.insert(
        "http://www.rs-ingenieure.de/de/hochbau/leistungen/tragwerksplanung",
        "rs-ingenieure.de.tragwerksplanung.html",
    );
    m.insert(
        "http://www.simplyscience.ch/teens-liesnach-archiv/articles/wie-entsteht-erdoel.html",
        "simplyscience.ch.erdoel.html",
    );
    m.insert(
        "http://www.shingon-reiki.de/reiki-und-schamanismus/",
        "shingon-reiki.de.schamanismus.html",
    );
    m.insert(
        "http://love-hina.ch/news/0409.html",
        "love-hina.ch.0409.html",
    );
    m.insert(
        "http://www.cdu-fraktion-erfurt.de/inhalte/aktuelles/entwicklung-der-waldorfschule-ermoeglicht/index.html",
        "cdu-fraktion-erfurt.de.waldorfschule.html",
    );
    m.insert(
        "http://www.wehranlage-horka.de/veranstaltung/887/",
        "wehranlage-horka.de.887.html",
    );
    m.insert(
        "https://de.creativecommons.org/index.php/2014/03/20/endlich-wird-es-spannend-die-nc-einschraenkung-nach-deutschem-recht/",
        "de.creativecommons.org.endlich.html",
    );
    m.insert(
        "https://piratenpartei-mv.de/blog/2013/09/12/grundeinkommen-ist-ein-menschenrecht/",
        "piratenpartei-mv.de.grundeinkommen.html",
    );
    m.insert(
        "https://scilogs.spektrum.de/engelbart-galaxis/die-ablehnung-der-gendersprache/",
        "spektrum.de.engelbart.html",
    );
    m.insert(
        "https://www.rnz.de/nachrichten_artikel,-zz-dpa-Schlaglichter-Frank-Witzel-erhaelt-Deutschen-Buchpreis-2015-_arid,133484.html",
        "rnz.de.witzel.html",
    );
    m.insert(
        "https://www.austria.info/de/aktivitaten/radfahren/radfahren-in-der-weltstadt-salzburg",
        "austria.info.radfahren.html",
    );
    m.insert(
        "https://buchperlen.wordpress.com/2013/10/20/leandra-lou-der-etwas-andere-modeblog-jetzt-auch-zwischen-buchdeckeln/",
        "buchperlen.wordpress.com.html",
    );
    m.insert("https://www.fairkom.eu/about", "fairkom.eu.about.html");
    m.insert(
        "https://futurezone.at/digital-life/uber-konkurrent-lyft-startet-mit-waymo-robotertaxis-in-usa/400487461",
        "futurezone.at.lyft.html",
    );
    m.insert(
        "http://www.hundeverein-kreisunna.de/unserverein.html",
        "hundeverein-kreisunna.de.html",
    );
    m.insert(
        "https://viehbacher.com/de/steuerrecht",
        "viehbacher.com.steuerrecht.html",
    );
    m.insert(
        "http://www.jovelstefan.de/2011/09/11/gefallt-mir/",
        "jovelstefan.de.gefallt.html",
    );
    m.insert(
        "https://www.stuttgart.de/item/show/132240/1",
        "stuttgart.de.html",
    );
    m.insert(
        "https://www.modepilot.de/2019/05/21/geht-euch-auch-so-oder-auf-reisen-nie-ohne-meinen-duschkopf/",
        "modepilot.de.duschkopf.html",
    );
    m.insert(
        "https://www.otto.de/twoforfashion/strohtasche/",
        "otto.de.twoforfashion.html",
    );
    m.insert(
        "http://iloveponysmag.com/2018/05/24/barbour-coastal/",
        "iloveponysmag.com.barbour.html",
    );
    m.insert(
        "https://moritz-meyer.net/blog/vreni-frost-instagram-abmahnung/",
        "moritz-meyer.net.vreni.html",
    );
    m.insert(
        "http://www.womencantalksports.com/top-10-women-talking-sports/",
        "womencantalksports.com.top10.html",
    );
    m.insert(
        "https://plentylife.blogspot.com/2017/05/strong-beautiful-pamela-reif-rezension.html",
        "plentylife.blogspot.pamela-reif.html",
    );
    m.insert(
        "https://www.luxuryhaven.co/2019/05/nam-nghi-phu-quoc-unbound-collection-by-hyatt-officially-opens.html",
        "luxuryhaven.co.hyatt.html",
    );
    m.insert(
        "https://www.luxuriousmagazine.com/2019/06/royal-salute-polo-rome/",
        "luxuriousmagazine.com.polo.html",
    );
    m.insert(
        "https://www.chip.de/tests/akkuschrauber-werkzeug-co,82197/5",
        "chip.de.tests.html",
    );
    m.insert(
        "https://www.gruen-digital.de/2015/01/digitalpolitisches-jahrestagung-2015-der-heinrich-boell-stiftung-baden-wuerttemberg/",
        "gruen-digital.de.jahrestagung.html",
    );
    m.insert(
        "https://www.rechtambild.de/2011/10/bgh-marions-kochbuch-de/",
        "rechtambild.de.kochbuch.html",
    );
    m.insert(
        "http://www.internet-law.de/2011/07/verstost-der-ausschluss-von-pseudonymen-bei-google-gegen-deutsches-recht.html",
        "internet-law.de.pseudonymen.html",
    );
    m.insert(
        "https://www.telemedicus.info/article/2766-Rezension-Haerting-Internetrecht,-5.-Auflage-2014.html",
        "telemedicus.info.rezension.html",
    );
    m.insert(
        "https://www.cnet.de/88130484/so-koennen-internet-user-nach-dem-eugh-urteil-fuer-den-schutz-sensibler-daten-sorgen",
        "cnet.de.schutz.html",
    );
    m.insert(
        "https://correctiv.org/aktuelles/neue-rechte/2019/05/14/wir-haben-bereits-die-zusage",
        "correctiv.org.zusage.html",
    );
    m.insert(
        "https://www.sueddeutsche.de/wirtschaft/bahn-flixbus-flixtrain-deutschlandtakt-fernverkehr-1.4445845",
        "sueddeutsche.de.flixtrain.html",
    );
    m.insert(
        "https://www.adac.de/rund-ums-fahrzeug/tests/kindersicherheit/kindersitztest-2018/",
        "adac.de.kindersitze.html",
    );
    m.insert(
        "https://www.caktusgroup.com/blog/2015/06/08/testing-client-side-applications-django-post-mortem/",
        "caktusgroup.com.django.html",
    );
    m.insert(
        "https://www.computerbase.de/2007-06/htc-touch-bald-bei-o2-als-xda-nova/",
        "computerbase.de.htc.html",
    );
    m.insert(
        "http://www.chineselyrics4u.com/2011/07/zhi-neng-xiang-nian-ni-jam-hsiao-jing.html",
        "chineselyrics4u.com.zhineng.html",
    );
    m.insert(
        "https://www.basicthinking.de/blog/2018/12/05/erfolgreiche-tweets-zutaten/",
        "basicthinking.de.tweets.html",
    );
    m.insert(
        "https://meedia.de/2016/03/08/einstieg-ins-tv-geschaeft-wie-freenet-privatkunden-fuer-antennen-tv-in-hd-qualitaet-gewinnen-will/",
        "meedia.de.freenet.html",
    );
    m.insert(
        "https://www.incurvy.de/trends-grosse-groessen/wellness-gesichtsbehandlung-plaisir-daromes/",
        "incurvy.de.wellness.html",
    );
    m.insert(
        "https://www.dw.com/en/uncork-the-mystery-of-germanys-fr%C3%BChburgunder/a-16863843",
        "dw.com.uncork.html",
    );
    m.insert(
        "https://www.jolie.de/stars/adele-10-kilo-abgenommen-sie-zeigt-sich-schlanker-denn-je-200226.html",
        "jolie.de.adele.html",
    );
    m.insert(
        "https://www.speicherguide.de/digitalisierung/faktor-mensch/schwierige-gespraeche-so-gehts-24376.aspx",
        "speicherguide.de.schwierige.html",
    );
    m.insert(
        "https://novalanalove.com/ear-candy/",
        "novalanalove.com.ear-candy.html",
    );
    m.insert(
        "http://www.franziska-elea.de/2019/02/10/das-louis-vuitton-missgeschick/",
        "franziska-elea.de.vuitton.html",
    );
    m.insert(
        "https://www.gofeminin.de/abnehmen/wie-kann-ich-schnell-abnehmen-s1431651.html",
        "gofeminin.de.abnehmen.html",
    );
    m.insert(
        "https://www.brigitte.de/liebe/persoenlichkeit/ikigai-macht-dich-sofort-gluecklicher--10972896.html",
        "brigitte.de.ikigai.html",
    );
    m.insert(
        "https://www.changelog.blog/zwischenbilanz-jan-kegelberg-ueber-tops-und-flops-bei-der-transformation-von-sportscheck/",
        "changelog.blog.zwischenbilanz.html",
    );
    m.insert(
        "https://threatpost.com/android-ransomware-spreads-via-sex-simulation-game-links-on-reddit-sms/146774/",
        "threatpost.com.android.html",
    );
    m.insert(
        "https://www.vice.com/en_uk/article/d3avvm/the-amazon-is-on-fire-and-the-smoke-can-be-seen-from-space",
        "vice.com.amazon.html",
    );
    m.insert(
        "https://www.heise.de/newsticker/meldung/Lithium-aus-dem-Schredder-4451133.html",
        "heise.de.lithium.html",
    );
    m.insert(
        "https://www.theverge.com/2019/7/3/20680681/ios-13-beta-3-facetime-attention-correction-eye-contact",
        "theverge.com.ios13.html",
    );
    m.insert(
        "https://crazy-julia.de/beauty-tipps-die-jede-braut-kennen-sollte/",
        "crazy-julia.de.tipps.html",
    );
    m.insert(
        "https://www.politische-bildung-brandenburg.de/themen/land-und-leute/homo-brandenburgensis",
        "brandenburg.de.homo-brandenburgensis.html",
    );
    m.insert(
        "https://skateboardmsm.de/news/the-captains-quest-2017-contest-auf-schwimmender-miniramp-am-19-august-in-dormagen.html",
        "skateboardmsm.de.dormhagen.html",
    );
    m.insert(
        "https://knowtechie.com/rocket-pass-4-in-rocket-league-brings-with-it-a-new-rally-inspired-car/",
        "knowtechie.com.rally.html",
    );
    m.insert(
        "https://boingboing.net/2013/07/19/hating-millennials-the-preju.html",
        "boingboing.net.millenials.html",
    );
    m.insert(
        "https://en.wikipedia.org/wiki/T-distributed_stochastic_neighbor_embedding",
        "en.wikipedia.org.tsne.html",
    );
    m.insert(
        "https://mixed.de/vrodo-deals-vr-taugliches-notebook-fuer-83215-euro-99-cent-leihfilme-bei-amazon-psvr/",
        "mixed.de.vrodo.html",
    );
    m.insert(
        "http://www.spreeblick.com/blog/2006/07/29/aus-aus-alles-vorbei-habeck-macht-die-stahnke/",
        "spreeblick.com.habeck.html",
    );
    m.insert(
        "https://majkaswelt.com/top-5-fashion-must-haves-2018-werbung/",
        "majkaswelt.com.fashion.html",
    );
    m.insert(
        "https://erp-news.info/erp-interview-mit-um-digitale-assistenten-und-kuenstliche-intelligenz-ki/",
        "erp-news.info.interview.html",
    );
    m.insert(
        "https://github.blog/2019-03-29-leader-spotlight-erin-spiceland/",
        "github.blog.spiceland.html",
    );
    m.insert(
        "https://lady50plus.de/2019/06/19/sekre-mystery-bag/",
        "lady50plus.de.sekre.html",
    );
    m.insert(
        "https://www.sonntag-sachsen.de/emanuel-scobel-wird-thomanerchor-geschaeftsfuehrer",
        "sonntag-sachsen.de.emanuel.html",
    );
    m.insert(
        "https://www.psl.eu/actualites/luniversite-psl-quand-les-grandes-ecoles-font-universite",
        "psl.eu.luniversite.html",
    );
    m.insert(
        "https://www.chip.de/test/Beef-Maker-von-Aldi-im-Test_154632771.html",
        "chip.de.beef.html",
    );
    m.insert(
        "http://www.sauvonsluniversite.fr/spip.php?article8532",
        "sauvonsluniversite.com.spip.html",
    );
    m.insert(
        "https://www.spiegel.de/spiegel/print/d-161500790.html",
        "spiegel.de.albtraum.html",
    );
    m.insert(
        "https://lemire.me/blog/2019/08/02/json-parsing-simdjson-vs-json-for-modern-c/",
        "lemire.me.json.html",
    );
    m.insert(
        "https://www.zeit.de/mobilitaet/2020-01/zugverkehr-christian-lindner-hochgeschwindigkeitsstrecke-eu-kommission",
        "zeit.de.zugverkehr.html",
    );
    m.insert(
        "https://www.franceculture.fr/emissions/le-journal-des-idees/le-journal-des-idees-emission-du-mardi-14-janvier-2020",
        "franceculture.fr.idees.html",
    );
    m.insert(
        "https://wikimediafoundation.org/news/2020/01/15/access-to-wikipedia-restored-in-turkey-after-more-than-two-and-a-half-years/",
        "wikimediafoundation.org.turkey.html",
    );
    m.insert(
        "https://www.reuters.com/article/us-awards-sag/parasite-scores-upset-at-sag-awards-boosting-oscar-chances-idUSKBN1ZI0EH",
        "reuters.com.parasite.html",
    );
    m.insert(
        "https://vancouversun.com/technology/microsoft-moves-to-erase-its-carbon-footprint-from-the-atmosphere-in-climate-push/wcm/76e426d9-56de-40ad-9504-18d5101013d2",
        "vancouversun.com.microsoft.html",
    );
    m.insert(
        "https://www.lanouvellerepublique.fr/indre-et-loire/commune/saint-martin-le-beau/family-park-la-derniere-saison-a-saint-martin-le-beau",
        "lanouvellerepublique.fr.martin.html",
    );
    m.insert(
        "http://www.pcgamer.com/2012/08/09/skyrim-part-1/",
        "pcgamer.com.skyrim.html",
    );
    m
}
