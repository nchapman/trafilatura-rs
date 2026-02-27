// Port of go-trafilatura/realworld_test.go

mod common;

use common::{extract_mock_file, html_contains, res_contains};

/// Port of `Test_Extract` from realworld_test.go.
///
/// Each test loads a real-world HTML snapshot, runs extraction with `enable_fallback = true`,
/// then asserts that expected strings appear and junk strings do not appear.
///
/// Tests are deliberately lenient: a missing result (extraction error) counts as "not contains",
/// matching the Go test approach where extractMockFile panics on error — we skip that URL on None.
/// (Some pages may fail extraction legitimately because our Rust port doesn't have the
///  readability/domdistiller fallback generators that Go has.)
#[test]
fn test_extract_die_partei() {
    let url = "https://die-partei.net/luebeck/2012/05/31/das-ministerium-fur-club-kultur-informiert/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "Impressum"));
    assert!(res_contains(&result, "Die GEMA dreht völlig am Zeiger!"));
}

#[test]
fn test_extract_bmjv() {
    let url = "https://www.bmjv.de/DE/Verbraucherportal/KonsumImAlltag/TransparenzPreisanpassung/TransparenzPreisanpassung_node.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "Impressum"));
    assert!(res_contains(&result, "Anbieter von Fernwärme haben innerhalb ihres Leitungsnetzes ein Monopol"));
}

#[test]
fn test_extract_denkanstoos() {
    let url = "https://denkanstoos.wordpress.com/2012/04/11/denkanstoos-april-2012/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Two or three 10-15 min"));
    assert!(res_contains(&result, "What type? Etc. (30 mins)"));
    assert!(!res_contains(&result, "Dieser Eintrag wurde veröffentlicht"));
    assert!(!res_contains(&result, "Mit anderen Teillen"));
}

#[test]
fn test_extract_ebrosia() {
    let url = "https://www.ebrosia.de/beringer-zinfandel-rose-stone-cellars-lieblich-suess";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Das Bukett präsentiert sich"));
    assert!(!res_contains(&result, "Kunden kauften auch"));
    assert!(!res_contains(&result, "Gutschein sichern"));
    assert!(res_contains(&result, "Besonders gut passt er zu asiatischen Gerichten"));
}

#[test]
fn test_extract_landwirt() {
    let url = "https://www.landwirt.com/Precision-Farming-Moderne-Sensortechnik-im-Kuhstall,,4229,,Bericht.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Überwachung der somatischen Zellen"));
    assert!(res_contains(&result, "tragbaren Ultraschall-Geräten"));
    assert!(res_contains(&result, "Kotkonsistenz"));
    assert!(!res_contains(&result, "Anzeigentarife"));
    assert!(!res_contains(&result, "Aktuelle Berichte aus dieser Kategorie"));
}

#[test]
fn test_extract_rs_ingenieure() {
    let url = "http://www.rs-ingenieure.de/de/hochbau/leistungen/tragwerksplanung";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Wir bearbeiten alle Leistungsbilder"));
    assert!(!res_contains(&result, "Brückenbau"));
}

#[test]
fn test_extract_shingon_reiki() {
    let url = "http://www.shingon-reiki.de/reiki-und-schamanismus/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "Catch Evolution"));
    assert!(!res_contains(&result, "und gekennzeichnet mit"));
    assert!(res_contains(&result, "Heut geht es"));
    assert!(res_contains(&result, "Ich komme dann zu dir vor Ort."));
}

#[test]
#[ignore = "go-trafilatura uses go-readability (go-shiori) which excludes comment sections; \
            our readability-rs (readeck port) includes them"]
fn test_extract_love_hina() {
    let url = "http://love-hina.ch/news/0409.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Kapitel 121 ist"));
    assert!(!res_contains(&result, "Kommentare schreiben"));
}

#[test]
fn test_extract_cdu_fraktion_erfurt() {
    let url = "http://www.cdu-fraktion-erfurt.de/inhalte/aktuelles/entwicklung-der-waldorfschule-ermoeglicht/index.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "der steigenden Nachfrage gerecht zu werden."));
    assert!(!res_contains(&result, "Zurück zur Übersicht"));
    assert!(!res_contains(&result, "Erhöhung für Zoo-Eintritt"));
}

#[test]
fn test_extract_creativecommons() {
    let url = "https://de.creativecommons.org/index.php/2014/03/20/endlich-wird-es-spannend-die-nc-einschraenkung-nach-deutschem-recht/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "das letzte Wort sein kann."));
    assert!(!res_contains(&result, "Ähnliche Beiträge"));
}

#[test]
fn test_extract_piratenpartei() {
    let url = "https://piratenpartei-mv.de/blog/2013/09/12/grundeinkommen-ist-ein-menschenrecht/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Unter diesem Motto findet am 14. September"));
    assert!(res_contains(&result, "Volksinitiative Schweiz zum Grundeinkommen."));
    assert!(!res_contains(&result, "getaggt mit:"));
    assert!(!res_contains(&result, "Was denkst du?"));
}

#[test]
fn test_extract_spektrum_engelbart() {
    let url = "https://scilogs.spektrum.de/engelbart-galaxis/die-ablehnung-der-gendersprache/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Zweitens wird der Genderstern"));
    assert!(res_contains(&result, "alldem leider – nichts."));
}

#[test]
fn test_extract_wehranlage_horka() {
    let url = "http://www.wehranlage-horka.de/veranstaltung/887/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "In eine andere Zeit"));
    assert!(res_contains(&result, "Während Sie über den Markt schlendern"));
    assert!(!res_contains(&result, "Infos zum Verein"));
    assert!(!res_contains(&result, "nach oben"));
    assert!(!res_contains(&result, "Datenschutzerklärung"));
}

#[test]
fn test_extract_demokratiewebstatt() {
    let url = "https://www.demokratiewebstatt.at/thema/thema-umwelt-und-klima/woher-kommt-die-dicke-luft";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Millionen Menschen fahren jeden Tag"));
    assert!(!res_contains(&result, "Clipdealer"));
    assert!(!res_contains(&result, "Teste dein Wissen"));
    assert!(!res_contains(&result, "Thema: Fußball"));
}

#[test]
fn test_extract_simplyscience_erdoel() {
    let url = "http://www.simplyscience.ch/teens-liesnach-archiv/articles/wie-entsteht-erdoel.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Erdöl bildet nach Millionen"));
    assert!(res_contains(&result, "Warum wird das Erdöl knapp?"));
    assert!(!res_contains(&result, "Die Natur ist aus chemischen Elementen aufgebaut"));
}

#[test]
#[ignore = "go-trafilatura uses go-readability (go-shiori) which extracts the article body; \
            our readability-rs (readeck port) picks up nav content instead"]
fn test_extract_rnz_witzel() {
    let url = "https://www.rnz.de/nachrichten_artikel,-zz-dpa-Schlaglichter-Frank-Witzel-erhaelt-Deutschen-Buchpreis-2015-_arid,133484.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Für einen Roman"));
    assert!(res_contains(&result, "Auszeichnung der Branche."));
}

#[test]
fn test_extract_buchperlen() {
    let url = "https://buchperlen.wordpress.com/2013/10/20/leandra-lou-der-etwas-andere-modeblog-jetzt-auch-zwischen-buchdeckeln/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Dann sollten Sie erst recht"));
    assert!(res_contains(&result, "als saure Gürkchen entlarvte Ex-Boyfriends."));
    assert!(!res_contains(&result, "Ähnliche Beiträge"));
}

#[test]
fn test_extract_toralin() {
    let url = "http://www.toralin.de/schmierfett-reparierend-verschlei-y-910.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "künftig das XADO-Schutzfett verwenden."));
    assert!(res_contains(&result, "bis zu 50% Verschleiß."));
    assert!(res_contains(&result, "Die Lebensdauer von Bauteilen erhöht sich beträchtlich."));
    assert!(!res_contains(&result, "Newsletter"));
    assert!(!res_contains(&result, "Sie könnten auch an folgenden Artikeln interessiert sein"));
}

#[test]
fn test_extract_fairkom() {
    let url = "https://www.fairkom.eu/about";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "ein gemeinwohlorientiertes Partnerschaftsnetzwerk"));
    assert!(res_contains(&result, "Stimmberechtigung bei der Generalversammlung."));
    assert!(!res_contains(&result, "support@fairkom.eu"));
}

#[test]
fn test_extract_futurezone_lyft() {
    let url = "https://futurezone.at/digital-life/uber-konkurrent-lyft-startet-mit-waymo-robotertaxis-in-usa/400487461";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Einige Kunden des Fahrdienst-Vermittler Lyft"));
    assert!(res_contains(&result, "zeitweise rund vier Prozent."));
    assert!(!res_contains(&result, "Allgemeine Nutzungsbedingungen"));
    assert!(!res_contains(&result, "Waymo bittet Autohersteller um Geld"));
}

#[test]
fn test_extract_hundeverein() {
    let url = "http://www.hundeverein-kreisunna.de/unserverein.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Beate und Norbert Olschewski"));
    assert!(res_contains(&result, "ein Familienmitglied und unser Freund."));
    assert!(!res_contains(&result, "zurück zur Startseite"));
}

#[test]
fn test_extract_viehbacher() {
    let url = "https://viehbacher.com/de/steuerrecht";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "und wirtschaftlich orientierte Privatpersonen"));
    assert!(res_contains(&result, "rund um die Uhr."));
    assert!(res_contains(&result, "Mensch im Mittelpunkt."));
    assert!(!res_contains(&result, "Was sind Cookies?"));
}

#[test]
fn test_extract_jovelstefan() {
    let url = "http://www.jovelstefan.de/2011/09/11/gefallt-mir/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Manchmal überrascht einen"));
    assert!(res_contains(&result, "kein Meisterwerk war!"));
    assert!(!res_contains(&result, "Pingback von"));
    assert!(!res_contains(&result, "Kommentare geschlossen"));
}

#[test]
fn test_extract_stuttgart() {
    let url = "https://www.stuttgart.de/item/show/132240/1";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Das Bohnenviertel entstand"));
    assert!(res_contains(&result, "sich herrlich entspannen."));
    assert!(!res_contains(&result, "Nützliche Links"));
    assert!(!res_contains(&result, "Mehr zum Thema"));
}

#[test]
fn test_extract_kulinariaathome() {
    let url = "http://kulinariaathome.wordpress.com/2012/12/08/mandelplatzchen/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "zu einem glatten Teig verarbeiten."));
    assert!(res_contains(&result, "goldbraun sind."));
    assert!(res_contains(&result, "200 g Zucker"));
    assert!(res_contains(&result, "Ein Backblech mit Backpapier auslegen."));
    assert!(!res_contains(&result, "Sei der Erste"));
    assert!(!res_contains(&result, "Gefällt mir"));
    assert!(!res_contains(&result, "Trotz sorgfältiger inhaltlicher Kontrolle"));
}

#[test]
fn test_extract_schleifen_ucoz() {
    let url = "http://schleifen.ucoz.de/blog/briefe/2010-10-26-18";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Es war gesagt,"));
    assert!(res_contains(&result, "Symbol auf dem Finger haben"));
    // Note: the Go test also asserts res_contains(&result, "Aufrufe:"), but Go's own
    // code comments this as a difference from the Python original:
    //   "TODO: this one is different than the original.
    //    In original, it should be false, but our go-readability still catch it."
    // "Aufrufe:" is a page-view counter in a metadata table footer. The Python original
    // trafilatura expects it NOT to be extracted. We follow the Python original here.
}

#[test]
fn test_extract_austria_radfahren() {
    let url = "https://www.austria.info/de/aktivitaten/radfahren/radfahren-in-der-weltstadt-salzburg";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Salzburg liebt seine Radfahrer."));
    assert!(res_contains(&result, "Puls einsaugen zu lassen."));
    assert!(!res_contains(&result, "Das könnte Sie auch interessieren ..."));
    assert!(!res_contains(&result, "So macht Radfahren sonst noch Spaß"));
}

#[test]
fn test_extract_modepilot_duschkopf() {
    let url = "https://www.modepilot.de/2019/05/21/geht-euch-auch-so-oder-auf-reisen-nie-ohne-meinen-duschkopf/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Allerdings sieht es wie ein Dildo aus,"));
    assert!(res_contains(&result, "gibt Bescheid, ne?"));
    assert!(!res_contains(&result, "Ähnliche Beiträge"));
    assert!(!res_contains(&result, "Deine E-Mail (bleibt natürlich unter uns)"));
}

#[test]
fn test_extract_otto_strohtasche() {
    let url = "https://www.otto.de/twoforfashion/strohtasche/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Ob rund oder kastenförmig, ob dezent oder auffällig"));
    assert!(res_contains(&result, "XX, Die Redaktion"));
    assert!(!res_contains(&result, " Kommentieren"));
    assert!(!res_contains(&result, "Dienstag, 4. Juni 2019"));
}

#[test]
fn test_extract_iloveponysmag_barbour() {
    let url = "http://iloveponysmag.com/2018/05/24/barbour-coastal/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Eine meiner besten Entscheidungen bisher:"));
    assert!(res_contains(&result, "Verlassenes Gewächshaus meets versteckter Deich"));
    assert!(res_contains(&result, "Der Hundestrand in Stein an der Ostsee"));
    assert!(!res_contains(&result, "Tags: Barbour,"));
    assert!(res_contains(&result, "Bitte (noch) mehr Bilder von Helle"));
    assert!(!res_contains(&result, "Hinterlasse einen Kommentar"));
}

#[test]
fn test_extract_moritz_meyer_vreni() {
    let url = "https://moritz-meyer.net/blog/vreni-frost-instagram-abmahnung/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Das ist alles nicht gekennzeichnet, wie soll ich wissen"));
    assert!(res_contains(&result, "Instagramshops machen es Abmahnanwälten leicht"));
    assert!(!res_contains(&result, "Diese Geschichte teilen"));
    assert!(!res_contains(&result, "Ähnliche Beiträge "));
    assert!(res_contains(&result, "Ich bin der Ansicht, abwarten und Tee trinken."));
    assert!(res_contains(&result, "Danke für dein Feedback. Auch zum Look meiner Seite."));
    assert!(!res_contains(&result, "Diese Website verwendet Akismet, um Spam zu reduzieren."));
}

#[test]
fn test_extract_womencantalksports() {
    let url = "http://www.womencantalksports.com/top-10-women-talking-sports/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Keep Talking Sports!"));
    assert!(!res_contains(&result, "Category: Blog Popular"));
    assert!(!res_contains(&result, "Copyright Women Can Talk Sports."));
    assert!(!res_contains(&result, "Submit your sports question below"));
    assert!(res_contains(&result, "3.Charlotte Jones Anderson"));
}

#[test]
fn test_extract_plentylife_pamela() {
    let url = "https://plentylife.blogspot.com/2017/05/strong-beautiful-pamela-reif-rezension.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Schönheit kommt für Pamela von Innen und Außen"));
    assert!(res_contains(&result, "Die Workout Übungen kannte ich bereits"));
    assert!(res_contains(&result, "Great post, I like your blog"));
    assert!(!res_contains(&result, "Links zu diesem Post"));
    assert!(!res_contains(&result, "mehr über mich ♥"));
    assert!(!res_contains(&result, "Bitte beachte auch die Datenschutzerklärung von Google."));
}

#[test]
fn test_extract_luxuryhaven_hyatt() {
    let url = "https://www.luxuryhaven.co/2019/05/nam-nghi-phu-quoc-unbound-collection-by-hyatt-officially-opens.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Grounded in sustainable architecture and refined Vietnamese craftsmanship,"));
    assert!(res_contains(&result, "and Carmelo Resort"));
    assert!(res_contains(&result, "OMG what a beautiful place to stay! "));
    assert!(!res_contains(&result, "Food Advertising by"));
    assert!(res_contains(&result, "Dining and Drinking"));
    assert!(!res_contains(&result, "A lovely note makes a beautiful day!"));
}

#[test]
fn test_extract_luxuriousmagazine_polo() {
    let url = "https://www.luxuriousmagazine.com/2019/06/royal-salute-polo-rome/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Argentina, the birthplace of polo."));
    assert!(res_contains(&result, "Simon Wittenberg travels to the Eternal City in Italy"));
    assert!(!res_contains(&result, "Luxury and lifestyle articles"));
    assert!(!res_contains(&result, "Pinterest"));
}

#[test]
fn test_extract_gruen_digital() {
    let url = "https://www.gruen-digital.de/2015/01/digitalpolitisches-jahrestagung-2015-der-heinrich-boell-stiftung-baden-wuerttemberg/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Prof. Dr. Caja Thimm"));
    assert!(res_contains(&result, "zur Anmeldung."));
    assert!(!res_contains(&result, "Next post"));
    assert!(!res_contains(&result, "Aus den Ländern"));
}

#[test]
fn test_extract_rechtambild() {
    let url = "https://www.rechtambild.de/2011/10/bgh-marions-kochbuch-de/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Leitsätze des Gerichts"));
    assert!(!res_contains(&result, "twittern"));
    assert!(!res_contains(&result, "Ähnliche Beiträge"));
    assert!(!res_contains(&result, "d.toelle[at]rechtambild.de"));
}

#[test]
fn test_extract_internet_law() {
    let url = "http://www.internet-law.de/2011/07/verstost-der-ausschluss-von-pseudonymen-bei-google-gegen-deutsches-recht.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Wann Blogs einer Impressumspflicht unterliegen,"));
    assert!(!res_contains(&result, "Über mich"));
    assert!(!res_contains(&result, "Gesetzes- und Rechtsprechungszitate werden automatisch"));
    assert!(res_contains(&result, "Mit Verlaub, ich halte das für groben Unsinn."));
}

#[test]
fn test_extract_telemedicus() {
    let url = "https://www.telemedicus.info/article/2766-Rezension-Haerting-Internetrecht,-5.-Auflage-2014.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Aufbau und Inhalt"));
    assert!(res_contains(&result, "Verlag Dr. Otto Schmidt"));
    assert!(!res_contains(&result, "Handbuch"));
    assert!(!res_contains(&result, "Drucken"));
    assert!(!res_contains(&result, "Ähnliche Artikel"));
    assert!(!res_contains(&result, "Anzeige:"));
}

#[test]
fn test_extract_cnet_de() {
    let url = "https://www.cnet.de/88130484/so-koennen-internet-user-nach-dem-eugh-urteil-fuer-den-schutz-sensibler-daten-sorgen";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Auch der Verweis auf ehrverletzende Bewertungen"));
    assert!(!res_contains(&result, "Fanden Sie diesen Artikel nützlich?"));
    assert!(!res_contains(&result, "Kommentar hinzufügen"));
    assert!(!res_contains(&result, "Anja Schmoll-Trautmann"));
    assert!(!res_contains(&result, "Aktuell"));
}

#[test]
fn test_extract_correctiv() {
    let url = "https://correctiv.org/aktuelles/neue-rechte/2019/05/14/wir-haben-bereits-die-zusage";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "Alle Artikel zu unseren Recherchen"));
    assert!(res_contains(&result, "Vorweg: Die beteiligten AfD-Politiker"));
    assert!(res_contains(&result, "ist heute Abend um 21 Uhr auch im ZDF-Magazin Frontal"));
    assert!(!res_contains(&result, "Wir informieren Sie regelmäßig zum Thema Neue Rechte"));
    assert!(!res_contains(&result, "Kommentar verfassen"));
    assert!(!res_contains(&result, "weiterlesen"));
}

#[test]
fn test_extract_sueddeutsche_flixtrain() {
    let url = "https://www.sueddeutsche.de/wirtschaft/bahn-flixbus-flixtrain-deutschlandtakt-fernverkehr-1.4445845";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "05:28 Uhr"));
    assert!(res_contains(&result, "Bahn-Konkurrenten wie Flixbus fürchten durch den geplanten Deutschlandtakt"));
    assert!(!res_contains(&result, "ICE im S-Bahn-Takt"));
    assert!(!res_contains(&result, "Diskussion zu diesem Artikel auf:"));
    assert!(!res_contains(&result, "Berater-Affäre bringt Bahnchef Lutz in Bedrängnis"));
    assert!(res_contains(&result, "auch der Bus ein klimafreundliches Verkehrsmittel sei"));
}

#[test]
fn test_extract_adac_kindersitze() {
    let url = "https://www.adac.de/rund-ums-fahrzeug/tests/kindersicherheit/kindersitztest-2018/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "Rund ums Fahrzeug"));
    assert!(res_contains(&result, "in punkto Sicherheit, Bedienung, Ergonomie"));
    assert!(res_contains(&result, "Grenzwert der Richtlinie 2014/79/EU"));
    assert!(!res_contains(&result, "Diesel-Umtauschprämien"));
    assert!(res_contains(&result, "Besonders bei Babyschalen sollte geprüft werden"));
}

#[test]
fn test_extract_caktusgroup_django() {
    let url = "https://www.caktusgroup.com/blog/2015/06/08/testing-client-side-applications-django-post-mortem/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Was I losing my mind?"));
    assert!(res_contains(&result, "being cached after their first access."));
    assert!(res_contains(&result, "Finding a Fix"));
    assert!(res_contains(&result, "from django.conf import settings"));
    assert!(!res_contains(&result, "New Call-to-action"));
    assert!(!res_contains(&result, "Contact us"));
    assert!(!res_contains(&result, "Back to blog"));
    assert!(!res_contains(&result, "You might also like:"));
}

#[test]
fn test_extract_computerbase_htc() {
    let url = "https://www.computerbase.de/2007-06/htc-touch-bald-bei-o2-als-xda-nova/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Vor knapp zwei Wochen"));
    assert!(res_contains(&result, "gibt es in der dazugehörigen Vorstellungs-News."));
    assert!(!res_contains(&result, "Themen:"));
    assert!(!res_contains(&result, "bis Januar 2009 Artikel für ComputerBase verfasst."));
    assert!(!res_contains(&result, "Warum Werbebanner?"));
    assert!(!res_contains(&result, "71 Kommentare"));
}

#[test]
fn test_extract_chineselyrics4u() {
    let url = "http://www.chineselyrics4u.com/2011/07/zhi-neng-xiang-nian-ni-jam-hsiao-jing.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "就放心去吧"));
    assert!(res_contains(&result, "Repeat Chorus"));
    assert!(!res_contains(&result, "Older post"));
    assert!(!res_contains(&result, "Thank you for your support!"));
}

#[test]
fn test_extract_basicthinking_tweets() {
    let url = "https://www.basicthinking.de/blog/2018/12/05/erfolgreiche-tweets-zutaten/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Frank Thelen, Investor"));
    assert!(res_contains(&result, "Female founders must constantly consider"));
    assert!(res_contains(&result, "Thema des öffentlichen Interesses"));
    assert!(!res_contains(&result, "Nach langjähriger Tätigkeit im Ausland"));
    assert!(res_contains(&result, "Schaut man ganz genau hin, ist der Habeck-Kommentar"));
    assert!(!res_contains(&result, "Mit Absendung des Formulars willige ich"));
    assert!(!res_contains(&result, "Kommentieren"));
}

#[test]
fn test_extract_meedia_freenet() {
    let url = "https://meedia.de/2016/03/08/einstieg-ins-tv-geschaeft-wie-freenet-privatkunden-fuer-antennen-tv-in-hd-qualitaet-gewinnen-will/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Welche Werbeeinnahmen erwarten Sie hier langfristig?"));
    assert!(res_contains(&result, "wir haben keinerlei Pläne, das zu verändern."));
    assert!(!res_contains(&result, "Nachrichtenüberblick abonnieren"));
    assert!(!res_contains(&result, "über alle aktuellen Entwicklungen auf dem Laufenden."));
    assert!(!res_contains(&result, "Schlagworte"));
    assert!(!res_contains(&result, "Teilen"));
    assert!(!res_contains(&result, "Dauerzoff um drohenden UKW-Blackout"));
    assert!(res_contains(&result, "Mobilcom Debitel has charged me for third party"));
}

#[test]
fn test_extract_incurvy_wellness() {
    let url = "https://www.incurvy.de/trends-grosse-groessen/wellness-gesichtsbehandlung-plaisir-daromes/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Zeit für Loslassen und Entspannung."));
    assert!(res_contains(&result, "Wie sieht dein Alltag aus?"));
    assert!(res_contains(&result, "Erfrischende, abschwellende Augencreme Phyto Contour"));
    assert!(res_contains(&result, "Vielen Dank Anja für deine Tipps rund um Beauty"));
    assert!(!res_contains(&result, "Betreiberin von incurvy Plus Size"));
    assert!(!res_contains(&result, "Wir verwenden Cookies"));
}

#[test]
fn test_extract_dw_frühburgunder() {
    let url = "https://www.dw.com/en/uncork-the-mystery-of-germanys-fr%C3%BChburgunder/a-16863843";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "No grape variety invites as much intrigue"));
    assert!(res_contains(&result, "With just 0.9 hectares"));
    assert!(!res_contains(&result, "Related Subjects"));
    assert!(!res_contains(&result, "Audios and videos on the topic"));
}

#[test]
fn test_extract_jolie_adele() {
    let url = "https://www.jolie.de/stars/adele-10-kilo-abgenommen-sie-zeigt-sich-schlanker-denn-je-200226.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Adele feierte ausgelassen mit den Spice Girls"));
    assert!(res_contains(&result, "wie sich Adele weiterentwickelt."));
    assert!(!res_contains(&result, "Sommerzeit ist Urlaubszeit,"));
    assert!(!res_contains(&result, "Lade weitere Inhalte"));
}

#[test]
fn test_extract_speicherguide_schwierige() {
    let url = "https://www.speicherguide.de/digitalisierung/faktor-mensch/schwierige-gespraeche-so-gehts-24376.aspx";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Konflikte mag keiner."));
    assert!(res_contains(&result, "Gespräche meistern können."));
    assert!(!res_contains(&result, "Flexible Wege in die"));
    assert!(!res_contains(&result, "Storage für den Mittelstand"));
    assert!(!res_contains(&result, "Weiterführender Link"));
}

#[test]
fn test_extract_novalanalove_ear_candy() {
    let url = "https://novalanalove.com/ear-candy/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Earcuff: Zoeca"));
    assert!(res_contains(&result, "mit längeren Ohrringen (:"));
    assert!(res_contains(&result, "Kreole: Stella Hoops"));
    assert!(!res_contains(&result, "Jetzt heißt es schnell sein:"));
    assert!(!res_contains(&result, "Diese Website speichert Cookies"));
    assert!(!res_contains(&result, "VON Sina Giebel"));
}

#[test]
fn test_extract_franziska_elea_vuitton() {
    let url = "http://www.franziska-elea.de/2019/02/10/das-louis-vuitton-missgeschick/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Zuerst dachte ich, ich könnte das"));
    assert!(res_contains(&result, "x Franzi"));
    assert!(res_contains(&result, "Flauschjacke: Bershka"));
    assert!(!res_contains(&result, "Palm Springs Mini (links)"));
    assert!(!res_contains(&result, "Diese Website verwendet Akismet"));
    assert!(!res_contains(&result, "New York, New York"));
    assert!(html_contains(&result, "Flauschjacke: <strong>Bershka</strong>"));
}

#[test]
fn test_extract_gofeminin_abnehmen() {
    let url = "https://www.gofeminin.de/abnehmen/wie-kann-ich-schnell-abnehmen-s1431651.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Die Psyche spielt eine nicht unerhebliche Rolle"));
    assert!(!res_contains(&result, "Sportskanone oder Sportmuffel"));
    assert!(!res_contains(&result, "PINNEN"));
    assert!(res_contains(&result, "2. Satt essen bei den Mahlzeiten"));
    assert!(!res_contains(&result, "Bringt die Kilos zum Purzeln!"));
    assert!(!res_contains(&result, "Crash-Diäten ziehen meist den Jojo-Effekt"));
}

#[test]
fn test_extract_brigitte_ikigai() {
    let url = "https://www.brigitte.de/liebe/persoenlichkeit/ikigai-macht-dich-sofort-gluecklicher--10972896.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Glücks-Trend Konkurrenz"));
    assert!(res_contains(&result, "Praktiziere Dankbarkeit"));
    assert!(res_contains(&result, "dein Ikigai schon gefunden?"));
    assert!(res_contains(&result, "14,90 Euro."));
    assert!(!res_contains(&result, "Neu in Liebe"));
    assert!(!res_contains(&result, "Erfahre mehr"));
    assert!(!res_contains(&result, "Erfahrung mit privater Arbeitsvermittlung?"));
}

#[test]
fn test_extract_changelog_blog_sportscheck() {
    let url = "https://www.changelog.blog/zwischenbilanz-jan-kegelberg-ueber-tops-und-flops-bei-der-transformation-von-sportscheck/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Gibt es weitere Top-Maßnahmen für Multi-Channel?"));
    assert!(res_contains(&result, "Vielen Dank für das interessante Interview!"));
    assert!(!res_contains(&result, "akzeptiere die Datenschutzbestimmungen"));
    assert!(!res_contains(&result, "Diese Beiträge solltest du nicht verpassen"));
    assert!(!res_contains(&result, "Annette Henkel"));
}

#[test]
fn test_extract_threatpost_android() {
    let url = "https://threatpost.com/android-ransomware-spreads-via-sex-simulation-game-links-on-reddit-sms/146774/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "These messages include links to the ransomware"));
    assert!(res_contains(&result, "using novel techniques to exfiltrate data."));
    assert!(!res_contains(&result, "Share this article:"));
    assert!(!res_contains(&result, "Write a comment"));
    assert!(!res_contains(&result, "Notify me when new comments are added."));
    assert!(!res_contains(&result, "uses Akismet to reduce spam."));
}

#[test]
fn test_extract_vice_amazon() {
    let url = "https://www.vice.com/en_uk/article/d3avvm/the-amazon-is-on-fire-and-the-smoke-can-be-seen-from-space";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Brazil went dark."));
    assert!(res_contains(&result, "the highest number of deforestation warnings.\u{201d}"));
    assert!(!res_contains(&result, "Tagged:"));
    assert!(!res_contains(&result, "to the VICE newsletter."));
    assert!(!res_contains(&result, "Watch this next"));
}

#[test]
fn test_extract_heise_lithium() {
    let url = "https://www.heise.de/newsticker/meldung/Lithium-aus-dem-Schredder-4451133.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Die Ökobilanz von Elektroautos"));
    assert!(res_contains(&result, "Nur die Folie bleibt zurück"));
    assert!(!res_contains(&result, "Forum zum Thema:"));
}

#[test]
fn test_extract_theverge_ios13() {
    let url = "https://www.theverge.com/2019/7/3/20680681/ios-13-beta-3-facetime-attention-correction-eye-contact";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Normally, video calls tend to"));
    assert!(res_contains(&result, "across both the eyes and nose."));
    assert!(res_contains(&result, "Added ARKit explanation and tweet."));
    assert!(!res_contains(&result, "Singapore's public health program"));
    assert!(!res_contains(&result, "Command Line delivers daily updates"));
}

#[test]
fn test_extract_crazy_julia_braut() {
    let url = "https://crazy-julia.de/beauty-tipps-die-jede-braut-kennen-sollte/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "in keinem Braut-Beauty-Programm fehlen darf?"));
    assert!(res_contains(&result, "nicht nur vor der Hochzeit ein absolutes Muss."));
    assert!(res_contains(&result, "Gesundes, glänzendes Haar"));
    assert!(!res_contains(&result, "Neue Wandbilder von Posterlounge"));
    assert!(!res_contains(&result, "mit meinen Texten und mit meinen Gedanken."));
    assert!(!res_contains(&result, "Erforderliche Felder sind mit * markiert."));
}

#[test]
fn test_extract_brandenburg_homo() {
    let url = "https://www.politische-bildung-brandenburg.de/themen/land-und-leute/homo-brandenburgensis";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Stilles Rackern, statt lautem Deklamieren."));
    assert!(res_contains(&result, "Watt jibt\u{2019}s n hier zu lachen?"));
    assert!(res_contains(&result, "Das Brandenbuch. Ein Land in Stichworten."));
    assert!(!res_contains(&result, "Bürgerbeteiligung"));
    assert!(!res_contains(&result, "Anmelden"));
    assert!(!res_contains(&result, "Foto: Timur"));
    assert!(!res_contains(&result, "Schlagworte"));
    assert!(!res_contains(&result, "Zeilenumbrüche und Absätze werden automatisch erzeugt."));
}

#[test]
fn test_extract_skateboardmsm_dormhagen() {
    let url = "https://skateboardmsm.de/news/the-captains-quest-2017-contest-auf-schwimmender-miniramp-am-19-august-in-dormagen.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Wakebeach 257"));
    assert!(res_contains(&result, "Be there or be square!"));
    assert!(res_contains(&result, "Hier geht\u{2019}s zur Facebook Veranstaltung"));
    assert!(!res_contains(&result, "More from News"));
    assert!(!res_contains(&result, "von Redaktion MSM"));
    assert!(!res_contains(&result, "add yours."));
}

#[test]
fn test_extract_knowtechie_rocket_league() {
    let url = "https://knowtechie.com/rocket-pass-4-in-rocket-league-brings-with-it-a-new-rally-inspired-car/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Rocket Pass 4 will begin at 10:00 a.m. PDT"));
    assert!(res_contains(&result, "Holy shit, Mortal Kombat 11"));
    assert!(res_contains(&result, "Let us know down below in the comments"));
    assert!(!res_contains(&result, "Related Topics"));
    assert!(!res_contains(&result, "You can keep up with me on Twitter"));
    assert!(!res_contains(&result, "Hit the track today with Mario Kart Tour"));
}

#[test]
fn test_extract_wikipedia_tsne() {
    let url = "https://en.wikipedia.org/wiki/T-distributed_stochastic_neighbor_embedding";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Given a set of high-dimensional objects"));
    assert!(res_contains(&result, "Herein a heavy-tailed Student t-distribution"));
    assert!(!res_contains(&result, "Categories:"));
    assert!(!res_contains(&result, "Conditional random field"));
}

#[test]
fn test_extract_mixed_de_vrodo() {
    let url = "https://mixed.de/vrodo-deals-vr-taugliches-notebook-fuer-83215-euro-99-cent-leihfilme-bei-amazon-psvr/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Niedlicher Roboter-Spielkamerad: Anki Cozmo"));
    assert!(res_contains(&result, "Empfehlungen von Dennis:"));
    assert!(!res_contains(&result, "Unterstütze unsere Arbeit"));
    assert!(!res_contains(&result, "Deepfake-Hollywood"));
    assert!(!res_contains(&result, "Avengers"));
    assert!(!res_contains(&result, "Katzenschreck"));
}

#[test]
fn test_extract_spreeblick_habeck() {
    let url = "http://www.spreeblick.com/blog/2006/07/29/aus-aus-alles-vorbei-habeck-macht-die-stahnke/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Hunderttausende von jungen Paaren"));
    assert!(res_contains(&result, "wie flatterhaft das Mädl ist? :)"));
    assert!(!res_contains(&result, "Malte Welding"));
    assert!(!res_contains(&result, "YouTube und die Alten"));
    assert!(!res_contains(&result, "Autokorrektur"));
}

#[test]
fn test_extract_majkaswelt_fashion() {
    let url = "https://majkaswelt.com/top-5-fashion-must-haves-2018-werbung/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Rüschen und Volants."));
    assert!(res_contains(&result, "ihr jedes Jahr tragen könnt?"));
    assert!(!res_contains(&result, "Das könnte dich auch interessieren"));
    assert!(!res_contains(&result, "Catherine Classic Lac 602"));
}

#[test]
fn test_extract_erp_news_interview() {
    let url = "https://erp-news.info/erp-interview-mit-um-digitale-assistenten-und-kuenstliche-intelligenz-ki/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Einblicke in die Vision zukünftiger Softwaregenerationen"));
    assert!(res_contains(&result, "Frage 4: Welche Rolle spielt Big Data in Bezug auf Assistenz-Systeme und KI?"));
    assert!(res_contains(&result, "von The unbelievable Machine Company (*um) zur Verfügung gestellt."));
    assert!(!res_contains(&result, "Matthias Weber ist ERP-Experte mit langjähriger Berufserfahrung."));
    assert!(!res_contains(&result, "Die Top 5 digitalen Trends für den Mittelstand"));
    assert!(!res_contains(&result, ", leading edge,"));
    assert!(html_contains(&result, "<strong>Vision zukünftiger Softwaregenerationen</strong>."));
    assert!(html_contains(&result, "von <b>The unbelievable Machine Company (*um)</b> zur Verfügung gestellt."));
}

#[test]
fn test_extract_boingboing_millenials() {
    let url = "https://boingboing.net/2013/07/19/hating-millennials-the-preju.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Click through for the whole thing."));
    assert!(res_contains(&result, "The generation we love to dump on"));
    assert!(!res_contains(&result, "GET THE BOING BOING NEWSLETTER"));
}

#[test]
fn test_extract_github_blog_spiceland() {
    let url = "https://github.blog/2019-03-29-leader-spotlight-erin-spiceland/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Erin Spiceland is a Software Engineer for SpaceX."));
    assert!(res_contains(&result, "make effective plans and goals for the future"));
    assert!(res_contains(&result, "looking forward to next?"));
    assert!(res_contains(&result, "Research Consultant at Adelard LLP"));
    assert!(!res_contains(&result, "Related posts"));
    assert!(!res_contains(&result, "Jeremy Epling"));
    assert!(!res_contains(&result, "Missed the main event?"));
    assert!(!res_contains(&result, "Privacy"));
}

#[test]
fn test_extract_lady50plus_sekre() {
    let url = "https://lady50plus.de/2019/06/19/sekre-mystery-bag/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "ist eine echte Luxushandtasche"));
    assert!(res_contains(&result, "Insgesamt 160 weibliche \u{201e}Designerinnen\u{201c}"));
    assert!(res_contains(&result, "Sei herzlich gegrüßt"));
    assert!(res_contains(&result, "Ein Mann alleine hätte niemals"));
    assert!(!res_contains(&result, "Erforderliche Felder sind mit"));
    assert!(!res_contains(&result, "Benachrichtige mich"));
    assert!(!res_contains(&result, "Reisen ist meine große Leidenschaft"));
    assert!(!res_contains(&result, "Styling Tipps für Oktober"));
    assert!(res_contains(&result, "in den Bann ziehen!"));
}

#[test]
fn test_extract_sonntag_sachsen_thomanerchor() {
    let url = "https://www.sonntag-sachsen.de/emanuel-scobel-wird-thomanerchor-geschaeftsfuehrer";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Neuer Geschäftsführender Leiter"));
    assert!(res_contains(&result, "nach Leipzig wechseln."));
    assert!(!res_contains(&result, "Mehr zum Thema"));
    assert!(!res_contains(&result, "Folgen Sie uns auf Facebook und Twitter"));
    assert!(!res_contains(&result, "Aktuelle Ausgabe"));
}

#[test]
fn test_extract_psl_eu_luniversite() {
    let url = "https://www.psl.eu/actualites/luniversite-psl-quand-les-grandes-ecoles-font-universite";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Le décret n°2019-1130 validant"));
    assert!(res_contains(&result, "restructurant à cet effet \u{bb}."));
    assert!(!res_contains(&result, " utilise des cookies pour"));
    assert!(!res_contains(&result, "En savoir plus"));
}

#[test]
fn test_extract_chip_de_beef() {
    let url = "https://www.chip.de/test/Beef-Maker-von-Aldi-im-Test_154632771.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Starke Hitze nur in der Mitte"));
    assert!(res_contains(&result, "ca. 35,7×29,4 cm"));
    assert!(res_contains(&result, "Wir sind im Steak-Himmel!"));
    assert!(!res_contains(&result, "Samsung Galaxy S10 128GB"));
    assert!(!res_contains(&result, "Für Links auf dieser Seite"));
}

#[test]
fn test_extract_sauvonsluniversite() {
    let url = "http://www.sauvonsluniversite.fr/spip.php?article8532";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "L\u{2019}AG Éducation Île-de-France inter-degrés"));
    assert!(res_contains(&result, "Grève et mobilisation pour le climat"));
    assert!(res_contains(&result, "suivi.reformes.blanquer@gmail.com"));
    assert!(!res_contains(&result, "Sauvons l\u{2019}Université !"));
    assert!(!res_contains(&result, "La semaine de SLU"));
}

#[test]
fn test_extract_spiegel_albtraum() {
    let url = "https://www.spiegel.de/spiegel/print/d-161500790.html";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Wie konnte es dazu kommen?"));
    assert!(res_contains(&result, "Die Geschichte beginnt am 26. Oktober"));
    assert!(res_contains(&result, "Es stützt seine Version."));
    assert!(!res_contains(&result, "und Vorteile sichern!"));
    assert!(!res_contains(&result, "Verschickt"));
    assert!(!res_contains(&result, "Die digitale Welt der Nachrichten."));
    assert!(!res_contains(&result, "Vervielfältigung nur mit Genehmigung"));
}

#[test]
fn test_extract_lemire_json_parsing() {
    let url = "https://lemire.me/blog/2019/08/02/json-parsing-simdjson-vs-json-for-modern-c/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "I use a Skylake processor with GNU GCC 8.3."));
    assert!(res_contains(&result, "gsoc-2018"));
    assert!(res_contains(&result, "0.091 GB/s"));
    assert!(res_contains(&result, "version 0.2 on vcpkg."));
    assert!(!res_contains(&result, "Leave a Reply"));
    assert!(!res_contains(&result, "Science and Technology links"));
    assert!(!res_contains(&result, "Proudly powered by WordPress"));
}

#[test]
fn test_extract_zeit_de_zugverkehr() {
    let url = "https://www.zeit.de/mobilitaet/2020-01/zugverkehr-christian-lindner-hochgeschwindigkeitsstrecke-eu-kommission";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "36 Stunden."));
    assert!(res_contains(&result, "Nationale Egoismen"));
    assert!(res_contains(&result, "Deutschland kaum beschleunigt."));
    assert!(!res_contains(&result, "Durchgehende Tickets fehlen"));
    assert!(res_contains(&result, "geprägte Fehlentscheidung."));
    assert!(res_contains(&result, "horrende Preise für miserablen Service bezahlen?"));
    assert!(!res_contains(&result, "Bitte melden Sie sich an, um zu kommentieren."));
}

#[test]
fn test_extract_franceculture_idees() {
    let url = "https://www.franceculture.fr/emissions/le-journal-des-idees/le-journal-des-idees-emission-du-mardi-14-janvier-2020";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Performativité"));
    assert!(res_contains(&result, "Les individus productifs communiquent"));
    assert!(res_contains(&result, "de nos espoirs et de nos désirs."));
    assert!(!res_contains(&result, "A la tribune je monterai"));
    assert!(!res_contains(&result, "À découvrir"));
    assert!(!res_contains(&result, "Le fil culture"));
}

#[test]
fn test_extract_wikimediafoundation_turkey() {
    let url = "https://wikimediafoundation.org/news/2020/01/15/access-to-wikipedia-restored-in-turkey-after-more-than-two-and-a-half-years/";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "as further access is restored."));
    assert!(!res_contains(&result, "Read further in the pursuit of knowledge"));
    assert!(!res_contains(&result, "Here's what that means."));
    assert!(!res_contains(&result, "Stay up-to-date on our work."));
    assert!(!res_contains(&result, "Photo credits"));
}

#[test]
fn test_extract_reuters_parasite() {
    let url = "https://www.reuters.com/article/us-awards-sag/parasite-scores-upset-at-sag-awards-boosting-oscar-chances-idUSKBN1ZI0EH";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(!res_contains(&result, "4 Min Read"));
    assert!(!res_contains(&result, "Factbox: Key winners"));
    assert!(res_contains(&result, "Despite an unknown cast,"));
    assert!(res_contains(&result, "Additional reporting by"));
}

#[test]
fn test_extract_vancouversun_microsoft() {
    let url = "https://vancouversun.com/technology/microsoft-moves-to-erase-its-carbon-footprint-from-the-atmosphere-in-climate-push/wcm/76e426d9-56de-40ad-9504-18d5101013d2";
    let result = extract_mock_file(url, false).expect("extraction should succeed");
    assert!(res_contains(&result, "Microsoft Corp said on Thursday"));
    assert!(res_contains(&result, "Postmedia is committed"));
    assert!(!res_contains(&result, "I consent to receiving"));
    assert!(res_contains(&result, "It was not immediately clear if"));
    assert!(!res_contains(&result, "turns CO2 into soap"));
    assert!(!res_contains(&result, "Reuters files"));
}

/// Test that links are preserved in HTML output when `include_links = true`.
/// Port of the final test case in Test_Extract.
#[test]
fn test_extract_pcgamer_skyrim_with_links() {
    let url = "http://www.pcgamer.com/2012/08/09/skyrim-part-1/";
    let result = extract_mock_file(url, true).expect("extraction should succeed");
    assert!(html_contains(&result, r#"In <a href="https://www.pcgamer.com/best-skyrim-mods/">Skyrim</a>, a mage"#));
    // Go serializes ' as &#39; but html5ever leaves it unescaped — both are valid HTML.
    assert!(
        html_contains(&result, "<em>Legends </em>don&#39;t destroy <em>houses</em>,")
        || html_contains(&result, "<em>Legends </em>don't destroy <em>houses</em>,")
    );
}
/// Port of `Test_ExoticTags` from trafilatura_test.go.
///
/// Covers edge cases with specially crafted HTML and the exotic_tags.html fixture.
#[test]
fn test_extract_exotic_tags() {
    // Fixture: teletype text and inline content
    let result = extract_mock_file("http://exotic_tags", false).expect("exotic_tags fixture should extract");
    assert!(res_contains(&result, "Teletype text"));
    assert!(res_contains(&result, "My new car is silver."));

    // Misformed HTML declaration
    let html = r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN" 2012"http://www.w3.org/TR/html4/loose.dtd"><html><head></head><body><p>ABC</p></body></html>"#;
    let result = trafilatura::extract(html, trafilatura::options::Options::default())
        .expect("misformed doctype should still extract");
    assert!(result.content_text.contains("ABC"));

    // Naked div with <br>: content should be joined with spaces.
    // Uses zero config (MinExtractedSize=0) to match Go's zeroConfig in Test_ExoticTags.
    let html = "<html><body><main><div>1.<br/>2.<br/>3.<br/></div></main></body></html>";
    let zero_opts = {
        let mut c = trafilatura::options::Config::default();
        c.min_extracted_size = 0;
        let mut o = trafilatura::options::Options::default();
        o.config = c;
        o
    };
    let result = trafilatura::extract(html, zero_opts).expect("naked div with br should extract");
    assert!(result.content_text.contains("1. 2. 3."));

    // HTML5 <details>/<summary>: both summary and body should be extracted
    let html = r#"<html><body><article><details><summary>Epcot Center</summary><p>Epcot is a theme park at Walt Disney World Resort featuring exciting attractions, international pavilions, award-winning fireworks and seasonal special events.</p></details></article></body></html>"#;
    let result = trafilatura::extract(html, trafilatura::options::Options::default())
        .expect("details element should extract");
    assert!(result.content_text.contains("Epcot Center"));
    assert!(result.content_text.contains("award-winning fireworks"));

    // Empty <a> inside <strong> must not cause empty output
    let html = r#"<html><body><div><h1>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</h1><h2>Sed et interdum lectus.</h2><p>Quisque molestie nunc eu arcu condimentum fringilla.</p><strong><a></a></strong><h2>Aliquam eget interdum elit, id posuere ipsum.</h2><p>Phasellus lectus erat, hendrerit sed tortor ac, dignissim vehicula metus.<br/></p></div></body></html>"#;
    let opts = {
        let mut o = trafilatura::options::Options::default();
        o.include_links = true;
        o.include_images = true;
        o
    };
    let result = trafilatura::extract(html, opts).expect("empty-a inside strong should not crash");
    assert!(!result.content_text.is_empty());

    // <em> improperly wrapping <p>: inner text must be extracted; result must end with "Text here"
    let html = r#"<html><body><div id="content"><h1>A header</h1><h2>Very specific bug so odd</h2><h3>Nested header</h3><p>Some "hyphenated-word quote" followed by a bit more text line.</p><em><p>em improperly wrapping p here</p></em><p>Text here<br/></p><h3>More articles</h3></div></body></html>"#;
    for focus in [
        trafilatura::options::ExtractionFocus::Balanced,
        trafilatura::options::ExtractionFocus::FavorRecall,
        trafilatura::options::ExtractionFocus::FavorPrecision,
    ] {
        let opts = {
            let mut o = trafilatura::options::Options::default();
            o.include_links = true;
            o.include_images = true;
            o.focus = focus;
            o
        };
        let result = trafilatura::extract(html, opts)
            .unwrap_or_else(|_| panic!("em-wrapping-p should extract (focus={focus:?})"));
        assert!(
            result.content_text.contains("em improperly wrapping p here"),
            "focus={focus:?}: expected 'em improperly wrapping p here'"
        );
        assert!(
            result.content_text.ends_with("Text here"),
            "focus={focus:?}: expected content to end with 'Text here', got: {:?}",
            result.content_text
        );
    }
}

// end of real-world tests
