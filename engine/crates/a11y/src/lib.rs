//! pdfcraft-a11y — accessibility (L4): the Accessibility Checker's full check.
//!
//! [`check`] runs the 32 rules in Acrobat's seven categories (Document, Page Content, Forms,
//! Alternate Text, Tables, Lists, Headings) and returns, per rule, Passed, Failed, Needs manual
//! check or Skipped, with the elements or pages at fault. [`report_html`] writes the
//! accessibility report. Rules read the document only; the fixes that need edits (language,
//! title, tab order) are applied by the engine.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::collections::{BTreeSet, HashMap};

use pdfcraft_cos::Document;

mod alt;
mod autotag;
mod autotag_fields;
mod content;
mod headings;
mod report;
mod structure;
mod tags;

#[cfg(test)]
thread_local! {
    /// Marked-content stack entries read to decide which marks enclose an operator, for the tests
    /// that keep that constant per operator.
    static MARK_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Page content passes made to find figure bounds.
    static BOUNDS_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts `n` marked-content stack reads (tests only).
#[inline]
fn count_mark_reads(_n: usize) {
    #[cfg(test)]
    MARK_READS.with(|c| c.set(c.get().saturating_add(_n)));
}

pub use alt::{AltError, ArtifactError, Figure, figures, mark_decorative, mark_tag_artifact, set_alt};
pub use autotag::{AutoTagError, AutoTagResult, autotag_text};
pub use autotag_fields::tag_form_fields;
pub use headings::{Heading, headings};
pub use report::{REPORT_WORDS, report_html};
pub use tags::{TagContentItem, TagError, TagInfo, TagPlacement, TagProperty, move_tag, set_tag_property, tag_content, tags};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Document,
    PageContent,
    Forms,
    AlternateText,
    Tables,
    Lists,
    Headings,
}

impl Category {
    pub const ALL: [Category; 7] =
        [Category::Document, Category::PageContent, Category::Forms, Category::AlternateText, Category::Tables, Category::Lists, Category::Headings];

    pub fn label(self) -> &'static str {
        match self {
            Category::Document => "Dokumen",
            Category::PageContent => "Konten Halaman",
            Category::Forms => "Formulir",
            Category::AlternateText => "Teks Alternatif",
            Category::Tables => "Tabel",
            Category::Lists => "Daftar",
            Category::Headings => "Heading",
        }
    }
}

/// The 32 rules of the full check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    PermissionFlag,
    ImageOnly,
    TaggedPdf,
    LogicalReadingOrder,
    PrimaryLanguage,
    Title,
    Bookmarks,
    ColorContrast,
    TaggedContent,
    TaggedAnnotations,
    TabOrder,
    CharacterEncoding,
    TaggedMultimedia,
    ScreenFlicker,
    Scripts,
    TimedResponses,
    NavigationLinks,
    TaggedFormFields,
    FieldDescriptions,
    FiguresAltText,
    NestedAltText,
    AltTextAssociated,
    AltTextHidesAnnotation,
    OtherElementsAltText,
    TableRows,
    TableCells,
    TableHeaders,
    TableRegularity,
    TableSummary,
    ListItems,
    LblLBody,
    HeadingNesting,
}

impl Rule {
    pub const ALL: [Rule; 32] = [
        Rule::PermissionFlag,
        Rule::ImageOnly,
        Rule::TaggedPdf,
        Rule::LogicalReadingOrder,
        Rule::PrimaryLanguage,
        Rule::Title,
        Rule::Bookmarks,
        Rule::ColorContrast,
        Rule::TaggedContent,
        Rule::TaggedAnnotations,
        Rule::TabOrder,
        Rule::CharacterEncoding,
        Rule::TaggedMultimedia,
        Rule::ScreenFlicker,
        Rule::Scripts,
        Rule::TimedResponses,
        Rule::NavigationLinks,
        Rule::TaggedFormFields,
        Rule::FieldDescriptions,
        Rule::FiguresAltText,
        Rule::NestedAltText,
        Rule::AltTextAssociated,
        Rule::AltTextHidesAnnotation,
        Rule::OtherElementsAltText,
        Rule::TableRows,
        Rule::TableCells,
        Rule::TableHeaders,
        Rule::TableRegularity,
        Rule::TableSummary,
        Rule::ListItems,
        Rule::LblLBody,
        Rule::HeadingNesting,
    ];

    /// The stable id used by automation (`permission-flag`, `tagged-content`, …).
    pub fn id(self) -> &'static str {
        self.info().0
    }

    pub fn category(self) -> Category {
        self.info().1
    }

    /// The short name shown in the results.
    pub fn name(self) -> &'static str {
        self.info().2
    }

    /// What the rule checks, as the options dialog words it.
    pub fn description(self) -> &'static str {
        self.info().3
    }

    /// Whether only a person can judge it.
    pub fn manual(self) -> bool {
        matches!(
            self,
            Rule::LogicalReadingOrder | Rule::ColorContrast | Rule::ScreenFlicker | Rule::Scripts | Rule::TimedResponses | Rule::NavigationLinks
        )
    }

    /// Checked unless deselected (Acrobat leaves colour contrast off).
    pub fn on_by_default(self) -> bool {
        self != Rule::ColorContrast
    }

    /// A short explanation of why the rule matters and how to satisfy it ("Explain").
    pub fn explanation(self) -> &'static str {
        self.info().4
    }

    pub fn from_id(id: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|r| r.id() == id)
    }

    fn info(self) -> (&'static str, Category, &'static str, &'static str, &'static str) {
        use Category as C;
        match self {
            Rule::PermissionFlag => (
                "permission-flag",
                C::Document,
                "Bendera izin aksesibilitas",
                "Bendera izin aksesibilitas aktif",
                "Pengaturan keamanan harus mengizinkan teknologi bantu membaca konten (\"Aktifkan akses teks untuk perangkat pembaca layar\"). Ubah keamanan dokumen untuk mengizinkannya.",
            ),
            Rule::ImageOnly => (
                "image-only",
                C::Document,
                "PDF hanya gambar",
                "Dokumen bukan PDF hanya gambar",
                "Halaman yang hanya berupa gambar teks tidak dapat dibacakan atau dicari. Kenali teks (OCR) untuk menambah lapisan teks.",
            ),
            Rule::TaggedPdf => (
                "tagged-pdf",
                C::Document,
                "PDF bertag",
                "Dokumen adalah PDF bertag",
                "Tag memberi dokumen struktur logis yang diikuti teknologi bantu. Beri tag pada dokumen (Tag PDF secara otomatis).",
            ),
            Rule::LogicalReadingOrder => (
                "logical-reading-order",
                C::Document,
                "Urutan Baca Logis",
                "Struktur dokumen menyediakan urutan baca yang logis",
                "Periksa bahwa tag dibaca dalam urutan yang akan dibaca pembaca yang dapat melihat. Gunakan alat Urutan Baca atau panel Tag untuk memperbaikinya.",
            ),
            Rule::PrimaryLanguage => (
                "primary-language",
                C::Document,
                "Bahasa utama",
                "Bahasa teks ditentukan",
                "Pembaca layar memilih suaranya dari bahasa dokumen. Atur di Properti Dokumen > Lanjutan > Bahasa.",
            ),
            Rule::Title => (
                "title",
                C::Document,
                "Judul",
                "Judul dokumen tampil pada bilah judul",
                "Jendela harus menampilkan judul yang bermakna, bukan nama berkas. Atur judul dan Tampilan Awal > Tampilkan: Judul Dokumen.",
            ),
            Rule::Bookmarks => (
                "bookmarks",
                C::Document,
                "Markah",
                "Markah ada pada dokumen besar",
                "Dokumen lebih dari 20 halaman membutuhkan markah untuk navigasi. Tambahkan markah untuk bagian utama.",
            ),
            Rule::ColorContrast => (
                "color-contrast",
                C::Document,
                "Kontras warna",
                "Dokumen memiliki kontras warna yang sesuai",
                "Teks harus cukup kontras dengan latarnya untuk orang dengan penglihatan lemah. Periksa warnanya dengan mata atau alat kontras.",
            ),
            Rule::TaggedContent => (
                "tagged-content",
                C::PageContent,
                "Konten bertag",
                "Semua konten halaman bertag",
                "Semua yang ada pada halaman harus bertag atau ditandai sebagai artifact (dekorasi, header, footer). Beri tag pada konten yang tidak bertag.",
            ),
            Rule::TaggedAnnotations => (
                "tagged-annotations",
                C::PageContent,
                "Anotasi bertag",
                "Semua anotasi bertag",
                "Komentar, tautan, dan bidang harus berada dalam pohon tag agar diumumkan. Beri tag pada anotasi.",
            ),
            Rule::TabOrder => (
                "tab-order",
                C::PageContent,
                "Urutan tab",
                "Urutan tab konsisten dengan urutan struktur",
                "Halaman dengan tautan, komentar, atau bidang harus menavigasi tab sesuai urutan struktur. Perbaikannya mengatur urutan tab tiap halaman ke Gunakan Struktur Dokumen.",
            ),
            Rule::CharacterEncoding => (
                "character-encoding",
                C::PageContent,
                "Pengodean karakter",
                "Pengodean karakter yang andal disediakan",
                "Font harus memetakan karakternya ke Unicode agar teks dapat dibaca dan disalin. Perbaiki font atau buat ulang PDF.",
            ),
            Rule::TaggedMultimedia => (
                "tagged-multimedia",
                C::PageContent,
                "Multimedia bertag",
                "Semua objek multimedia bertag",
                "Video, suara, dan media kaya harus bertag agar diumumkan. Beri tag pada anotasi multimedia.",
            ),
            Rule::ScreenFlicker => (
                "screen-flicker",
                C::PageContent,
                "Kedipan layar",
                "Halaman tidak akan menyebabkan kedipan layar",
                "Animasi yang berkedip dapat menyebabkan kejang. Periksa skrip dan media dari kedipan.",
            ),
            Rule::Scripts => (
                "scripts",
                C::PageContent,
                "Skrip",
                "Tidak ada skrip yang tidak dapat diakses",
                "Konten berskrip harus dapat digunakan dengan keyboard dan teknologi bantu. Periksa skrip dokumen.",
            ),
            Rule::TimedResponses => (
                "timed-responses",
                C::PageContent,
                "Respons berwaktu",
                "Halaman tidak membutuhkan respons berwaktu",
                "Orang mungkin membutuhkan waktu lebih untuk merespons. Periksa bahwa tidak ada yang memiliki batas waktu.",
            ),
            Rule::NavigationLinks => (
                "navigation-links",
                C::PageContent,
                "Tautan navigasi",
                "Tautan navigasi tidak berulang",
                "Tautan navigasi yang berulang harus dapat dilewati. Periksa tautan yang muncul pada setiap halaman.",
            ),
            Rule::TaggedFormFields => (
                "tagged-form-fields",
                C::Forms,
                "Bidang formulir bertag",
                "Semua bidang formulir bertag",
                "Bidang formulir harus berada dalam pohon tag (tag Form) agar diumumkan. Beri tag pada bidang formulir.",
            ),
            Rule::FieldDescriptions => (
                "field-descriptions",
                C::Forms,
                "Deskripsi bidang",
                "Semua bidang formulir memiliki deskripsi",
                "Tiap bidang membutuhkan tooltip yang menjelaskan apa yang harus diisi. Tambahkan di Properti Bidang > Umum > Tooltip.",
            ),
            Rule::FiguresAltText => (
                "figures-alt-text",
                C::AlternateText,
                "Teks alternatif gambar",
                "Gambar membutuhkan teks alternatif",
                "Gambar membutuhkan teks alternatif yang menjelaskannya, atau harus ditandai dekoratif. Tambahkan teks alternatif ke tiap gambar.",
            ),
            Rule::NestedAltText => (
                "nested-alt-text",
                C::AlternateText,
                "Teks alternatif bersarang",
                "Teks alternatif yang tidak akan pernah dibaca",
                "Teks alternatif di dalam elemen yang sudah memiliki teks alternatif tidak pernah dibaca. Hapus teks alternatif bagian dalam.",
            ),
            Rule::AltTextAssociated => (
                "alt-text-associated",
                C::AlternateText,
                "Terkait dengan konten",
                "Teks alternatif harus terkait dengan suatu konten",
                "Teks alternatif pada elemen tanpa konten halaman tidak pernah terjangkau. Hapus atau kaitkan elemen itu ke konten.",
            ),
            Rule::AltTextHidesAnnotation => (
                "alt-text-hides-annotation",
                C::AlternateText,
                "Menyembunyikan anotasi",
                "Teks alternatif tidak boleh menyembunyikan anotasi",
                "Teks alternatif pada elemen yang berisi tautan atau bidang menyembunyikan anotasi itu dari teknologi bantu. Pindahkan teks alternatifnya.",
            ),
            Rule::OtherElementsAltText => (
                "other-elements-alt-text",
                C::AlternateText,
                "Teks alternatif elemen lain",
                "Elemen lain yang membutuhkan teks alternatif",
                "Rumus dan elemen non-teks serupa membutuhkan teks alternatif. Tambahkan teks alternatif padanya.",
            ),
            Rule::TableRows => (
                "table-rows",
                C::Tables,
                "Baris",
                "TR harus menjadi anak dari Table, THead, TBody, atau TFoot",
                "Baris tabel berada langsung dalam tabel atau kepala, badan, atau kakinya. Perbaiki tag tabel.",
            ),
            Rule::TableCells => {
                ("table-cells", C::Tables, "TH dan TD", "TH dan TD harus menjadi anak dari TR", "Sel tabel berada dalam baris. Perbaiki tag tabel.")
            }
            Rule::TableHeaders => (
                "table-headers",
                C::Tables,
                "Header",
                "Tabel harus memiliki header",
                "Sel header (TH) memberi tahu pendengar isi tiap kolom atau baris. Tandai sel header.",
            ),
            Rule::TableRegularity => (
                "table-regularity",
                C::Tables,
                "Keteraturan",
                "Tabel harus berisi jumlah kolom yang sama pada tiap baris dan jumlah baris yang sama pada tiap kolom",
                "Tabel yang tidak teratur membingungkan navigasi sel. Beri tiap baris jumlah sel yang sama (termasuk span).",
            ),
            Rule::TableSummary => (
                "table-summary",
                C::Tables,
                "Ringkasan",
                "Tabel harus memiliki ringkasan",
                "Ringkasan menjelaskan tujuan dan tata letak tabel. Tambahkan atribut Summary ke tiap tabel.",
            ),
            Rule::ListItems => ("list-items", C::Lists, "Item daftar", "LI harus menjadi anak dari L", "Item daftar berada dalam daftar. Perbaiki tag daftar."),
            Rule::LblLBody => (
                "lbl-lbody",
                C::Lists,
                "Lbl dan LBody",
                "Lbl dan LBody harus menjadi anak dari LI",
                "Label dan badan daftar berada dalam item daftar. Perbaiki tag daftar.",
            ),
            Rule::HeadingNesting => (
                "heading-nesting",
                C::Headings,
                "Penyarangan yang sesuai",
                "Penyarangan yang sesuai",
                "Level heading tidak boleh melompat (H1 diikuti H3). Beri tag ulang heading.",
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    Passed,
    Failed,
    /// Needs manual check.
    Manual,
    /// Not checked (deselected or skipped).
    Skipped,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Passed => "Lulus",
            Status::Failed => "Gagal",
            Status::Manual => "Perlu pemeriksaan manual",
            Status::Skipped => "Dilewati",
        }
    }
}

/// One problem: where it is and what is wrong.
///
/// The sentence comes in two forms. [`message`](Finding::message) is it, in English, ready to read.
/// [`pattern`](Finding::pattern) is the same sentence with a named hole where each value went, and
/// [`values`](Finding::values) is what filled them, so an interface can say it in another language
/// instead of showing the English one: it translates the pattern and puts the same values back.
/// `message` is always `pattern` with `values` in it, because [`Finding::worded`] builds it.
///
/// Every hole holds the document's own data — an element type, a font name, a count — and goes in
/// as it stands, with one exception: a hole named `{what}` holds another wording of ours, still with
/// its own holes in it. An interface translates that wording too and puts it in the frame first, and
/// then fills every hole in what it has; [`Finding::nested`] builds such a finding.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// 0-based page, when the problem is on a page.
    pub page: Option<usize>,
    /// Indirect structure element that caused the finding, when one is available.
    pub object: Option<pdfcraft_cos::ObjRef>,
    /// Bounds in PDF user space when the affected content can be located safely.
    pub bbox: Option<[f64; 4]>,
    pub message: String,
    /// The English sentence with a hole, such as `{p}` or `{what}`, where each value went.
    pub pattern: &'static str,
    /// What goes in each of `pattern`'s holes, by the hole's name (`"{p}"`, braces included).
    pub values: Vec<(&'static str, String)>,
}

impl Finding {
    /// A finding whose sentence says the same thing however the document is put together.
    pub fn plain(page: Option<usize>, sentence: &'static str) -> Self {
        Finding { page, object: None, bbox: None, message: sentence.to_owned(), pattern: sentence, values: Vec::new() }
    }

    /// A finding built from a wording and what goes in its holes. The English `message` is filled
    /// in here, so it and `pattern` can never drift apart.
    pub fn worded(page: Option<usize>, pattern: &'static str, values: Vec<(&'static str, String)>) -> Self {
        Finding { page, object: None, bbox: None, message: say(pattern, &values), pattern, values }
    }

    /// A finding that says where the problem is and, in `{what}`, what is wrong, each worded
    /// separately so the two read in whichever order the language reads them.
    ///
    /// `what` is one of our own wordings and may have holes of its own; they are filled from
    /// `values` along with the frame's. It goes in before the values do — it is ours, not the
    /// document's — which is what lets its holes be filled at all.
    pub fn nested(page: Option<usize>, pattern: &'static str, what: &'static str, mut values: Vec<(&'static str, String)>) -> Self {
        let message = say(&pattern.replace("{what}", what), &values);
        values.push(("{what}", what.to_owned()));
        Finding { page, object: None, bbox: None, message, pattern, values }
    }

    /// This finding as one sentence, said in the language `words` translates into.
    ///
    /// The frame is translated, the wording in `{what}` — if the frame has one — is translated and
    /// goes in, and only then does the document's own data, in a single pass, so nothing the
    /// document itself says can be read as a hole. With `&|s: &str| s.to_owned()` this is
    /// [`message`](Finding::message).
    pub fn said(&self, words: Words) -> String {
        let mut frame = words(self.pattern);
        if let Some((_, what)) = self.values.iter().find(|(name, _)| *name == "{what}") {
            frame = frame.replace("{what}", &words(what));
        }
        say(&frame, &self.values)
    }
}

/// How an interface says one of our wordings in its own language, holes and all: given the English
/// in [`WORDINGS`], the same wording in that language. [`in_english`] keeps the English.
pub type Words<'a> = &'a dyn Fn(&str) -> String;

/// The [`Words`] that translate nothing: the wording as it is written here.
pub fn in_english(wording: &str) -> String {
    wording.to_owned()
}

/// A wording with its holes filled.
///
/// A hole is a name in braces, such as `{p}`; what is not in `values` is left as it is written.
/// Values go in during a single pass over the wording, so a value that itself contains braces is
/// never read as a hole: the document's own words reach the screen as they stand.
///
/// An interface says a [`Finding`] in its own language with this, after translating the pattern
/// and, if there is one, the wording in `{what}`.
pub fn say(pattern: &str, values: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        let (before, from_brace) = rest.split_at(open);
        out.push_str(before);
        // `find` counts from the brace, and `}` is one byte, so this is a char boundary; `get`
        // rather than slicing in case it ever isn't.
        let Some(close) = from_brace.find('}') else {
            // A brace with nothing closing it is just text.
            out.push_str(from_brace);
            return out;
        };
        let Some(hole) = from_brace.get(..=close) else { return out + from_brace };
        match values.iter().find(|(name, _)| *name == hole) {
            Some((_, value)) => out.push_str(value),
            None => out.push_str(hole),
        }
        rest = from_brace.get(close + 1..).unwrap_or_default();
    }
    out.push_str(rest);
    out
}

/// Every sentence the checker can say: each [`Finding::pattern`] and each wording that goes in a
/// `{what}` hole, in English, with its holes still in it.
///
/// An interface that shows the report in another language translates these, which is the whole
/// reason they are gathered here; `wordings_are_complete` fails if a rule says something that is
/// not on the list.
pub const WORDINGS: &[&str] = &[
    // Document rules.
    "Pengaturan keamanan tidak mengizinkan pembaca layar membaca teks",
    "Tidak ada halaman yang memiliki teks: dokumen tampaknya berupa gambar halaman",
    "Dokumen tidak memiliki tag (pohon struktur)",
    "Dokumen tidak ditandai sebagai bertag (MarkInfo Marked)",
    "Bahasa dokumen tidak diatur",
    "Dokumen tidak memiliki judul",
    "Jendela menampilkan nama berkas, bukan judul (Initial View > Show)",
    "{n} halaman dan tidak ada markah",
    // Page content and forms.
    "Dokumen tidak bertag, sehingga tidak ada kontennya yang bertag",
    "Halaman {p}: {n} item konten tidak bertag",
    "Halaman {p}: {n} item konten tidak bertag",
    "Bidang formulir pada halaman {p} tidak bertag",
    "Anotasi {} pada halaman {p} tidak bertag",
    "Halaman {p} tidak menavigasi tab sesuai urutan struktur",
    "Font {} pada halaman {p} tidak memetakan karakternya ke Unicode",
    "Font (tanpa nama) pada halaman {p} tidak memetakan karakternya ke Unicode",
    "Bidang \"{}\" pada halaman {p} tidak memiliki deskripsi (tooltip)",
    // The structure rules: the two frames, then what goes in their `{what}`.
    "Elemen {ty}: {what}",
    "Elemen {ty} pada halaman {p}: {what}",
    "tidak ada teks alternatif",
    "teks alternatifnya berada di dalam elemen lain dan tidak akan pernah dibaca",
    "teks alternatif pada elemen tanpa konten halaman",
    "teks alternatif menyembunyikan anotasi di dalamnya",
    "tidak berada dalam Table, THead, TBody, atau TFoot",
    "tidak berada dalam TR",
    "tidak ada sel header (TH)",
    "baris memiliki antara {min} dan {max} kolom",
    "tidak ada ringkasan",
    "tidak berada dalam L (daftar)",
    "tidak berada dalam LI (item daftar)",
    "heading pertama adalah H{n}; heading tidak boleh melompati level",
    "H{n} mengikuti H{d}; heading tidak boleh melompati level",
    // The summary at the end of a long list.
    AND_MORE,
];

#[derive(Clone, Debug, PartialEq)]
pub struct RuleResult {
    pub rule: Rule,
    pub status: Status,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The rules to run; the others are reported as Skipped.
    pub rules: BTreeSet<Rule>,
    /// 0-based pages for the page rules (`None`: all pages).
    pub pages: Option<Vec<usize>>,
}

impl Default for Options {
    fn default() -> Self {
        Self { rules: Rule::ALL.into_iter().filter(|r| r.on_by_default()).collect(), pages: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub results: Vec<RuleResult>,
}

impl Report {
    pub fn result(&self, rule: Rule) -> Option<&RuleResult> {
        self.results.iter().find(|r| r.rule == rule)
    }

    pub fn count(&self, status: Status) -> usize {
        self.results.iter().filter(|r| r.status == status).count()
    }

    /// Results of one category, in rule order.
    pub fn in_category(&self, c: Category) -> impl Iterator<Item = &RuleResult> {
        self.results.iter().filter(move |r| r.rule.category() == c)
    }
}

/// Findings beyond this many per rule are summarised ("… and n more").
const MAX_FINDINGS: usize = 200;

/// What the summary of the findings beyond [`MAX_FINDINGS`] says.
const AND_MORE: &str = "… dan {n} lainnya";

/// Run the full check.
pub fn check(doc: &Document, options: &Options) -> Report {
    let pages = pdfcraft_model::pages(doc);
    let page_list: Vec<usize> = match &options.pages {
        Some(p) => p.iter().copied().filter(|p| *p < pages.len()).collect(),
        None => (0..pages.len()).collect(),
    };
    let tagged = structure::has_struct_tree(doc);
    let scan = content::Scan::run(doc, &pages, &page_list);
    let mut tree_rules = structure::run(doc, &pages, &options.rules);
    let mut figure_bounds: Option<HashMap<pdfcraft_cos::ObjRef, (usize, [f64; 4])>> = None;
    let mut results = Vec::with_capacity(32);
    for rule in Rule::ALL {
        if !options.rules.contains(&rule) {
            results.push(RuleResult { rule, status: Status::Skipped, findings: Vec::new() });
            continue;
        }
        if rule.manual() {
            results.push(RuleResult { rule, status: Status::Manual, findings: Vec::new() });
            continue;
        }
        // `total` counts every finding; `findings` holds the first MAX_FINDINGS of them.
        let (mut findings, total) = match rule.category() {
            Category::Document => {
                let found = content::document_rule(doc, rule, &pages, &scan);
                let n = found.len();
                (found, n)
            }
            Category::PageContent | Category::Forms => {
                let found = content::page_rule(doc, rule, &pages, &page_list, &scan, tagged);
                let n = found.len();
                (found, n)
            }
            _ => tree_rules.remove(&rule).unwrap_or_default(),
        };
        if rule.category() == Category::AlternateText && findings.iter().any(|f| f.object.is_some()) {
            let bounds = figure_bounds.get_or_insert_with(|| {
                figures(doc)
                    .into_iter()
                    .filter_map(|figure| {
                        let (page, bbox) = (figure.page?, figure.bbox?);
                        (bbox.iter().all(|v| v.is_finite()) && bbox[0] < bbox[2] && bbox[1] < bbox[3]).then_some((figure.obj, (page, bbox)))
                    })
                    .collect()
            });
            for finding in &mut findings {
                if let Some((page, bbox)) = finding.object.and_then(|obj| bounds.get(&obj))
                    && finding.page == Some(*page)
                {
                    finding.bbox = Some(*bbox);
                }
            }
        }
        // A page read only in part may hold what the scan didn't find: text that makes the
        // document more than images, untagged content, or a font without Unicode mappings.
        let unsure = !scan.incomplete.is_empty()
            && match rule {
                Rule::ImageOnly => !scan.has_text,
                Rule::TaggedContent | Rule::CharacterEncoding => findings.is_empty(),
                _ => false,
            };
        if unsure {
            results.push(RuleResult { rule, status: Status::Manual, findings: Vec::new() });
            continue;
        }
        if total > MAX_FINDINGS {
            let more = total - MAX_FINDINGS;
            findings.truncate(MAX_FINDINGS);
            findings.push(Finding::worded(None, AND_MORE, vec![("{n}", more.to_string())]));
        }
        let status = if findings.is_empty() { Status::Passed } else { Status::Failed };
        results.push(RuleResult { rule, status, findings });
    }
    Report { results }
}

#[cfg(test)]
mod tests;
