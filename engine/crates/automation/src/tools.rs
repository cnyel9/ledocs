//! The tool table: names, descriptions and JSON Schemas for every automation tool.
//!
//! Tool names use `[a-z_]` only (MCP clients reject dots). `command` links a tool to the
//! registry id it automates, so `command_list` can tell an agent which tool runs a menu command.

use serde_json::{Value, json};

use crate::ToolError;

#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// JSON Schema (draft 2020-12 subset) for the arguments object.
    pub input_schema: Value,
    /// Does not change any document or file.
    pub read_only: bool,
    /// May remove content or overwrite files.
    pub destructive: bool,
    /// The registry command this tool automates, if any.
    pub command: Option<&'static str>,
}

fn doc() -> Value {
    json!({ "type": "integer", "minimum": 1, "description": "Id dokumen dari doc_open atau doc_list." })
}

fn pages(what: &str) -> Value {
    json!({ "type": "array", "items": { "type": "integer", "minimum": 1 }, "minItems": 1, "description": format!("Nomor halaman berbasis 1 {what}.") })
}

fn path(what: &str) -> Value {
    json!({ "type": "array", "items": { "type": "integer", "minimum": 1 }, "description": format!("{what}: posisi berbasis 1 dari tingkat teratas, mis. [2, 1] = anak pertama dari markah kedua.") })
}

fn path_arg() -> Value {
    json!({ "type": "string", "description": "Jalur berkas (relatif terhadap --root bila diatur)." })
}

fn point() -> Value {
    json!({ "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": "[x, y] dalam poin dari kiri atas halaman yang ditampilkan." })
}

fn measure_points() -> Value {
    json!({"type":"array","items":point(),"minItems":1,"maxItems":2048})
}
fn measure_schema() -> Value {
    schema(
        json!({"doc":doc(),"page":{"type":"integer","minimum":1},"points":measure_points(),"label":{"type":"string","maxLength":512},"author":{"type":"string","maxLength":512}}),
        &["doc", "page", "points"],
    )
}

fn color() -> Value {
    json!({ "type": "string", "description": "#RRGGBB atau nama warna: yellow, red, orange, green, blue, purple, pink, black, gray, white." })
}

/// Properties that pick one comment: its `id` (from comment_list), or `page` + `index`.
fn comment_ref(mut extra: Value) -> Value {
    if let Some(props) = extra.as_object_mut() {
        props.insert("doc".into(), doc());
        props.insert(
            "id".into(),
            json!({ "type": "string", "description": "Id komentar (dari comment_list). Id yang dipakai bersama oleh beberapa komentar ditolak: isi page dan index." }),
        );
        props.insert("page".into(), json!({ "type": "integer", "minimum": 1, "description": "Dengan `index`, sebagai pengganti `id`." }));
        props.insert(
            "index".into(),
            json!({ "type": "integer", "minimum": 1, "description": "Posisi berbasis 1 pada halaman, seperti yang dilaporkan comment_list." }),
        );
    }
    extra
}

fn schema(props: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false })
}

struct T {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    read_only: bool,
    destructive: bool,
    command: Option<&'static str>,
}

const fn t(name: &'static str, title: &'static str, description: &'static str) -> T {
    T { name, title, description, read_only: false, destructive: false, command: None }
}

impl T {
    const fn ro(mut self) -> Self {
        self.read_only = true;
        self
    }
    const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
    const fn cmd(mut self, id: &'static str) -> Self {
        self.command = Some(id);
        self
    }
    fn with(self, input_schema: Value) -> ToolDef {
        ToolDef {
            name: self.name,
            title: self.title,
            description: self.description,
            input_schema,
            read_only: self.read_only,
            destructive: self.destructive,
            command: self.command,
        }
    }
}

/// Every tool, in a stable order.
pub fn tools() -> Vec<ToolDef> {
    let save_out = json!({ "type": "string", "description": "Berkas tujuan penulisan. Kosongkan untuk membuka hasil sebagai dokumen baru yang belum disimpan." });
    let open = json!({ "type": "boolean", "description": "Buka juga hasil sebagai dokumen baru (bawaan: hanya bila out dikosongkan)." });
    vec![
        t("doc_batch", "Proses berkas dan folder", "Jalankan convert, ocr, dan/atau optimize pada berkas dan folder, satu berkas dalam satu waktu. recursive bawaan false. Pertahankan jalur relatif terhadap tiap folder terpilih di dalam out_dir; masukan ganda dijalankan sekali. Lewati berkas yang tidak didukung, tautan, dan subpohon keluaran. Keluaran yang sudah ada menerima nama bernomor dan tidak pernah diganti. Lanjutkan sesudah galat per berkas kecuali stop_on_error bernilai true. Kembalikan jumlah selesai/gagal/dilewati dan hasil per berkas. Penemuan dibatasi 10000 entri dan 64 tingkat folder. Convert harus pertama; tiap langkah dapat muncul sekali. OCR memakai model yang terpasang. Optimize memakai pengaturan Reduce File Size, dengan opsi mengganti ppi target (ambang 1.5 kali ppi) dan kualitas JPEG; kandidat yang lebih besar mempertahankan byte asli. Sumber dan dokumen terbuka tidak berubah.")
            .with(schema(json!({
                "paths": {"type":"array","items":{"type":"string"},"minItems":1,"maxItems":1000},
                "out_dir": {"type":"string"},
                "steps": {"type":"array","items":{"type":"string","enum":["convert","ocr","optimize"]},"minItems":1,"maxItems":3,"uniqueItems":true},
                "recursive": {"type":"boolean"}, "stop_on_error": {"type":"boolean"},
                "ppi": {"type":"number","minimum":9,"maximum":1600},
                "quality": {"type":"integer","minimum":1,"maximum":100},
                "dpi": {"type":"number","minimum":72,"maximum":600},
                "language": {"type":"string"}, "skip_text_pages": {"type":"boolean"}
            }), &["paths","out_dir","steps"])),
        t("attachment_add", "Lampirkan berkas", "Lampirkan berkas pada path ke dokumen (tingkat dokumen, panel Attachments; bukan komentar klip kertas pada halaman). name bawaan nama berkas; kunci lampiran, unik dalam dokumen, dikembalikan. Dapat dibatalkan.")
            .cmd("edit.attach")
            .with(schema(json!({"doc": doc(), "path": {"type": "string"}, "name": {"type": "string"}, "description": {"type": "string"}}), &["doc", "path"])),
        t("attachment_delete", "Hapus lampiran", "Hapus lampiran tingkat dokumen dengan nama ini (seperti yang dicantumkan doc_info). Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({"doc": doc(), "name": {"type": "string"}}), &["doc", "name"])),
        t("doc_open", "Buka PDF", "Buka berkas PDF dan kembalikan id dokumen, jumlah halaman, dan apakah dokumen dapat diedit.")
            .cmd("file.open")
            .with(schema(json!({ "path": { "type": "string" }, "password": { "type": "string", "description": "Kata sandi pengguna atau pemilik untuk berkas terenkripsi." } }), &["path"])),
        t("doc_list", "Daftar dokumen terbuka", "Daftar dokumen terbuka beserta id, jumlah halaman, dan status belum disimpan.").ro().with(schema(json!({}), &[])),
        t("doc_info", "Periksa dokumen", "Metadata, ukuran dan label halaman, markah, anotasi, bidang formulir, tautan, lapisan, transisi halaman dan waktu tampil, tujuan awal saat dibuka, lampiran, font, keamanan, dan catatan perbaikan.")
            .ro()
            .cmd("file.properties")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("doc_close", "Tutup dokumen", "Tutup dokumen. Gagal bila ada perubahan belum disimpan kecuali discard_changes bernilai true.")
            .cmd("file.close")
            .with(schema(json!({ "doc": doc(), "discard_changes": { "type": "boolean" } }), &["doc"])),
        t(
            "doc_save",
            "Simpan dokumen",
            "Simpan ke berkasnya sendiri (pembaruan inkremental, yang menjaga tanda tangan tetap sah) atau ke jalur baru (penulisan ulang penuh). Penulisan bersifat atomik. flatten_fill_sign membakar teks, tanda, dan tanda tangan Fill & Sign ke halaman terlebih dahulu dan menghapus anotasi tersebut; komentar dan bidang formulir lain tetap. Kosongkan, atau isi false, agar tetap dapat diedit.",
        )
        .destructive()
        .cmd("file.save")
        .with(schema(
            json!({
                "doc": doc(),
                "path": { "type": "string", "description": "Simpan sebagai berkas ini. Kosongkan untuk menyimpan di tempat." },
                "full": { "type": "boolean", "description": "Paksa penulisan ulang penuh (atau, dengan false, pembaruan inkremental)." },
                "flatten_fill_sign": { "type": "boolean", "description": "Bakar tanda Fill & Sign ke halaman sebelum menulis. Bawaan false." }
            }),
            &["doc"],
        )),
        t("doc_set_info", "Atur metadata dokumen", "Atur entri informasi dokumen seperti Title, Author, Subject, atau Keywords, atau properti khusus dengan nama lain apa pun (tanpa spasi, pemisah, / atau #; maksimal 127 byte; doc_info mencantumkannya di bawah custom); nilai kosong menghapusnya. Bila berkas memiliki metadata XMP, properti XMP yang cocok (dc:title, dc:creator, dc:description, pdf:Keywords, xmp:CreatorTool, pdf:Producer) ikut diubah, sehingga penampil yang membaca XMP lebih dulu menampilkan nilai baru. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "key": { "type": "string" }, "value": { "type": "string" } }), &["doc", "key", "value"])),
        t("page_render", "Render halaman", "Render satu halaman menjadi gambar PNG (bawaan 96 dpi, maksimal 600).")
            .ro()
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "dpi": { "type": "number", "minimum": 1, "maximum": 600 },
                "hide_text_block": { "type": "integer", "minimum": 1, "description": "Render latar editor sebaris tanpa paragraf ini (nomor dari text_paragraphs, diselesaikan seperti text_edit: ditolak bila satu edit menggabungkan paragraf itu ke paragraf tetangga). Tidak mengubah dokumen." },
                "hide_added_text": { "type": "integer", "minimum": 1, "description": "Render latar pengetikan ulang tanpa item teks tambahan ini (indeks added dari object_list). Saling eksklusif dengan hide_text_block. Tidak mengubah dokumen." }
            }), &["doc", "page"])),
        t("object_list", "Daftar objek yang dapat diedit", "Daftar paragraf yang ada, karya Image/Form, dan konten tambahan satu kali, dengan referensi jenis/indeks sumber, persegi yang ditampilkan, dan generasi dokumen. Indeks berubah saat konten berubah; gunakan generation pada object_move untuk menolak referensi basi.")
            .ro().with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1}}), &["doc","page"])),
        t("object_move", "Pindahkan objek bersama", "Geser secara atomik 1–1000 referensi campuran dari object_list sebesar [dx, dy] dalam poin tampilan (kanan/bawah). Mempertahankan kode glyph, font, data gambar, dan jarak relatif; keberhasilan adalah satu langkah batal. Pemotongan dan teks vertikal ditolak. Referensi yang buruk/basi menolak seluruh pemindahan.")
            .with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1},
                "objects":{"type":"array","minItems":1,"maxItems":1000,"uniqueItems":true,"items":{
                    "type":"object","properties":{"kind":{"type":"string","enum":["added","text","image"]},"index":{"type":"integer","minimum":1}},"required":["kind","index"],"additionalProperties":false}},
                "offset":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2},
                "generation":{"type":"integer","minimum":0,"description":"Gunakan generation yang dikembalikan object_list untuk menolak pilihan basi."}}), &["doc","page","objects","offset"])),
        t("text_word_count", "Jumlah kata", "Hitung kata dan karakter Unicode pada halaman terpilih (bawaan semua), dengan total dan jumlah per halaman. Pengecualian tabel/kode memakai tag struktur PDF, bukan tebakan visual; teks yang tidak terklasifikasi dipertahankan. Pengecualian gambar menghapus teks bertag Figure dan teks tidak terlihat/OCR. Tidak menjalankan OCR. Izin menyalin diperlukan. Gagal daripada mengembalikan total sebagian bila halaman tidak dapat dibaca.")
            .ro()
            .cmd("file.word_count")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang dihitung (bawaan: semua)"),
                "exclude_tables": { "type": "boolean" }, "exclude_code": { "type": "boolean" }, "exclude_images": { "type": "boolean" }
            }), &["doc"])),
        t("text_extract", "Ekstrak teks", "Ekstrak teks sebagian atau semua halaman, dalam urutan baca. Dengan rect, hanya teks di dalam persegi itu pada tiap halaman, seperti yang diambil Column select: satu baris per baris visual, potongan berdampingan (sel tabel) dipisahkan tab.")
            .ro()
            .cmd("edit.column_select")
            .with(schema(
                json!({
                    "doc": doc(), "pages": pages("yang diekstrak (bawaan: semua)"),
                    "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4, "description": "Column select: [x0, y0, x1, y1] dalam poin dari kiri atas halaman yang ditampilkan; glyph terhitung bila titik tengahnya di dalam." }
                }),
                &["doc"],
            )),
        t("text_find", "Cari teks", "Cari frasa (tidak peka huruf besar kecil, spasi dinormalkan) dan kembalikan kecocokan dalam urutan halaman dengan persegi halaman dan baris dalam poin (titik asal kiri atas). Membaca batch yang bertambah, melihat satu kecocokan melewati batas untuk kelanjutan. Halaman yang gagal tidak membuang kecocokan yang dapat dibaca: warnings berisi hingga 100 entri {page, error, error_truncated} (galat dibatasi 1024 byte UTF-8), warning_count menghitung halaman yang gagal, dan warnings_truncated melaporkan daftar yang dibatasi. complete bernilai false sesudah kegagalan halaman atau bila masih ada kecocokan lain; next hanya ada untuk kelanjutan batas kecocokan.")
            .ro()
            .cmd("edit.find")
            .with(schema(json!({ "doc": doc(), "query": { "type": "string", "minLength": 1 }, "limit": { "type": "integer", "minimum": 1, "description": "Kecocokan maksimum (bawaan 500). Bila ada kecocokan lain, next memuat start_page dan skip untuk melanjutkan. complete bernilai false baik untuk kecocokan tambahan maupun halaman yang gagal; pencarian yang habis dengan warnings tidak memiliki next." },
                "start_page": { "type": "integer", "minimum": 1, "description": "Halaman awal pencarian (bawaan 1)." },
                "skip": { "type": "integer", "minimum": 0, "description": "Kecocokan yang dilewati pada start_page sebelum mengembalikan apa pun (bawaan 0)." } }), &["doc", "query"])),
        t("page_rotate", "Putar halaman", "Putar halaman sebesar kelipatan 90 derajat (positif berarti searah jarum jam): halaman yang dicantumkan (bawaan semua), disaring seperti Rotate Pages pada Acrobat menurut subset (all, even, odd untuk nomor halaman) dan orientation (all, landscape, portrait). Dapat dibatalkan.")
            .cmd("page.rotate")
            .with(schema(
                json!({
                    "doc": doc(), "pages": pages("yang diputar (bawaan: semua)"), "degrees": { "type": "integer" },
                    "subset": { "type": "string", "enum": ["all", "even", "odd"] },
                    "orientation": { "type": "string", "enum": ["all", "landscape", "portrait"] },
                }),
                &["doc", "degrees"],
            )),
        t("page_enhance_camera", "Tingkatkan gambar kamera", "Luruskan dokumen hasil foto (Scan & OCR ▸ Enhance ▸ Camera image): bagian halaman di dalam empat sudut (x1, y1, … x4, y4 dalam poin bertitik asal kiri atas, urutan apa pun, segiempat cembung; bila dikosongkan, tepi dokumen ditemukan otomatis, atau galat menyatakan tidak dapat ditemukan) dilengkungkan menjadi persegi tegak yang menggantikan tampilan halaman, pada resolusi asli gambar halaman (maksimal 600 dpi). mode adalah tampilannya: original, colour (bawaan), greyscale, atau bw (hitam putih, disimpan tanpa kehilangan). level (0–100, bawaan 0) lalu membersihkan foto: pencahayaan merata sehingga kertas menjadi putih (semburat warna, bayangan, tepi gelap), kontras lebih tinggi, dan mulai 60 tidak ada bintik terisolasi; tinta berwarna tetap berwarna. Untuk bw, level adalah ambang (lebih tinggi menjaga goresan lebih samar tetap hitam). Komentar, tautan, dan markah halaman tetap. Dapat dibatalkan.")
            .cmd("ocr.enhance_camera")
            .with(schema(
                json!({
                    "doc": doc(),
                    "page": { "type": "integer", "minimum": 1 },
                    "corners": { "type": "array", "items": { "type": "number" }, "minItems": 8, "maxItems": 8 },
                    "level": { "type": "integer", "minimum": 0, "maximum": 100 },
                    "mode": { "type": "string", "enum": ["original", "colour", "greyscale", "bw"] },
                }),
                &["doc", "page"],
            )),
        t("photo_find_document", "Temukan dokumen dalam foto", "Pindai: sudut dokumen (lembaran, kartu, kuitansi) dalam foto pada path (JPEG atau PNG), sebelum menjadi halaman, agar dapat ditampilkan dan disesuaikan: corners [x1, y1, … x4, y4] sebagai proporsi (0–1) dari lebar dan tinggi foto saat diambil (orientasi EXIF diterapkan), kiri atas, kanan atas, kanan bawah, kiri bawah. found bernilai false bila tidak ada tepi dokumen yang meyakinkan.")
            .ro()
            .with(schema(json!({ "path": { "type": "string" } }), &["path"])),
        t("page_delete", "Hapus halaman", "Hapus halaman. Dapat dibatalkan sampai disimpan.")
            .destructive()
            .cmd("page.delete")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang dihapus") }), &["doc", "pages"])),
        t("page_move", "Pindahkan halaman", "Pindahkan halaman sehingga halaman pertama darinya berada pada posisi `to` (berbasis 1, dihitung sebelum pemindahan). Dapat dibatalkan.").with(schema(
            json!({ "doc": doc(), "pages": pages("yang dipindahkan"), "to": { "type": "integer", "minimum": 1 } }),
            &["doc", "pages", "to"],
        )),
        t("page_reverse", "Balikkan urutan halaman", "Balikkan urutan halaman di antara posisi yang ditempatinya: halaman yang dicantumkan (minimal dua), atau seluruh dokumen bila pages dikosongkan. Dapat dibatalkan.")
            .cmd("page.reverse")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang dibalik (bawaan: semua)") }), &["doc"])),
        t("page_insert_blank", "Sisipkan halaman kosong", "Sisipkan halaman kosong sehingga menjadi halaman `at`. Ukuran bawaan mengikuti halaman tetangga. Dapat dibatalkan.")
            .cmd("page.insert_blank")
            .with(schema(
                json!({ "doc": doc(), "at": { "type": "integer", "minimum": 1 }, "width": { "type": "number", "description": "Poin." }, "height": { "type": "number", "description": "Poin." } }),
                &["doc", "at"],
            )),
        t("page_insert_file", "Sisipkan halaman dari berkas", "Sisipkan halaman PDF lain sehingga halaman pertama menjadi halaman `at`. Markahnya ikut, diciutkan di bawah satu markah bernama sesuai berkas, ditempatkan di antara markah dokumen tempat halaman masuk. Dapat dibatalkan.")
            .cmd("page.insert")
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" }, "at": { "type": "integer", "minimum": 1 }, "pages": pages("dari berkas sumber (bawaan: semua)") }), &["doc", "path", "at"])),
        t("page_extract", "Ekstrak halaman", "Salin halaman ke PDF baru (tautan, markah, bidang, dan lapisan yang menjadi miliknya ikut). separate: true menulis tiap halaman sebagai berkas sendiri ke out_dir; delete: true menghapus halaman dari dokumen ini sesudahnya (dapat dibatalkan).")
            .cmd("page.extract")
            .with(schema(
                json!({ "doc": doc(), "pages": pages("yang diekstrak"), "out": save_out.clone(), "open": open.clone(), "separate": { "type": "boolean" }, "out_dir": { "type": "string" }, "delete": { "type": "boolean" } }),
                &["doc", "pages"],
            )),
        t("doc_combine", "Gabungkan berkas", "Gabungkan berkas, secara berurutan, menjadi satu PDF (markah disimpan di bawah satu entri per berkas). Gambar (PNG, JPEG, TIFF, GIF, BMP, JPEG 2000), berkas .txt, dan dokumen Word/Excel/PowerPoint/RTF (.docx, .xlsx, .pptx, .rtf) dijadikan PDF terlebih dahulu. pages secara opsional memilih halaman tiap berkas, selaras dengan paths: rentang seperti \"1-3, 6\" atau null untuk semua halaman. passwords, juga selaras dengan paths, membuka berkas terenkripsi (kata sandi buka, atau kata sandi izin bila keamanan berkas tidak mengizinkan penyalinan halaman; null bila tidak ada). Hasil tidak dienkripsi. Kata sandi tidak pernah ditampilkan kembali. groups, juga selaras dengan paths, membagi berkas di sekitar berkas lain: entri dengan nomor sama (harus menamai path yang sama) adalah satu berkas, disalin sekali, sehingga mempertahankan satu markah pada halaman pertamanya, tautan, dan bidang formulirnya; null untuk berkas tersendiri. Grup memakai kata sandi entri pertamanya. Bila formulir dua berkas menamai bidang yang sama, field_conflicts menyatakan yang terjadi: \"rename\" (bawaan) menjaga bidang tetap terpisah dengan mengganti nama yang berikutnya (Text1 menjadi Text1-1) dan hasil mencantumkan renamed_fields sebagai [[old, new], …], skrip yang merujuk nama lama tidak ditulis ulang; \"merge\" menjadikannya satu bidang, seperti sebelumnya.")
            .cmd("page.combine")
            .with(schema(
                json!({
                    "paths": { "type": "array", "items": { "type": "string" }, "minItems": 2 },
                    "pages": { "type": "array", "items": { "type": ["string", "null"] } },
                    "passwords": { "type": "array", "items": { "type": ["string", "null"] } },
                    "groups": { "type": "array", "items": { "type": ["integer", "null"], "minimum": 0 } },
                    "field_conflicts": { "type": "string", "enum": ["rename", "merge"] },
                    "out": save_out,
                    "open": open,
                }),
                &["paths"],
            )),
        t("doc_create_multiple", "Buat PDF dari banyak berkas", "Konversi beberapa berkas (PDF, gambar PNG/JPEG/JPEG 2000/TIFF/GIF/BMP, berkas .txt, dan dokumen Word .docx, Excel .xlsx, PowerPoint .pptx, dan .rtf, dideteksi dari konten dan dikonversi pada perangkat seperti doc_create `office`) menjadi PDF dalam satu proses. mode \"combine\" (bawaan) menggabungkannya, secara berurutan, menjadi satu PDF dengan satu markah per berkas; pages secara opsional memilih halaman tiap berkas, selaras dengan paths (rentang seperti \"1-3, 6\" atau null). mode \"separate\" menulis satu PDF per berkas ke out_dir (berkas yang sudah berupa PDF dilewati; berkas yang ada tidak pernah ditimpa) dan melaporkan hasil tiap berkas.")
            .cmd("create.multiple")
            .with(schema(
                json!({
                    "paths": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
                    "mode": { "type": "string", "enum": ["combine", "separate"] },
                    "pages": { "type": "array", "items": { "type": ["string", "null"] } },
                    "out": save_out.clone(),
                    "open": open.clone(),
                    "out_dir": { "type": "string", "description": "Folder untuk mode \"separate\"." },
                }),
                &["paths"],
            )),
        t("doc_split", "Pisahkan dokumen", "Pisahkan menjadi beberapa berkas yang ditulis ke out_dir: tiap N halaman, sebelum halaman tertentu, pada markah tingkat teratas (bookmarks: true; berkas dinamai menurut markah, judul berulang mendapat -2, -3), atau menurut ukuran berkas (max_mb). Berkas bernama <name>-partK.pdf.")
            .cmd("page.split")
            .with(schema(
                json!({ "doc": doc(), "every": { "type": "integer", "minimum": 1 }, "before": pages("yang memulai bagian baru"), "bookmarks": { "type": "boolean" }, "max_mb": { "type": "number", "exclusiveMinimum": 0 }, "out_dir": { "type": "string" } }),
                &["doc", "out_dir"],
            )),
        t(
            "bookmark_list",
            "Daftar markah",
            "Markah dalam urutan kerangka, satu halaman dalam satu waktu: masing masing dengan path (berbasis 1, seperti yang diterima tool markah lain), judul, halaman tujuan dan status terbuka, warna ([r, g, b], 0-1) serta gaya tebal atau miring bila ada, dan anaknya pada halaman yang sama. next adalah offset halaman berikutnya, atau null pada akhir. Markah yang bersarang lebih dalam dari 33 tingkat tidak dicantumkan.",
        )
        .ro()
        .with(schema(
            json!({
                "doc": doc(),
                "offset": { "type": "integer", "minimum": 0, "description": "Posisi awal halaman, dihitung dari 0 (next dari halaman sebelumnya)" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Markah dalam halaman (bawaan 100)" }
            }),
            &["doc"],
        )),
        t(
            "bookmark_add",
            "Tambah markah",
            "Tambah markah yang menuju halaman, di bawah markah induk (atau pada tingkat teratas), pada satu posisi. Markah menuju bagian atas halaman, atau tempat pada halaman: `top` dan `left` adalah poin dari kiri atas halaman yang ditampilkan (seperti tujuan yang dilaporkan bookmark_list), masing masing opsional, dan koordinat yang dikosongkan mempertahankan posisi pembaca pada arah itu. Zoom dipertahankan. Dapat dibatalkan.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "title": { "type": "string", "minLength": 1 },
                "page": { "type": "integer", "minimum": 1 },
                "parent": path("Markah induk (kosongkan untuk tingkat teratas)"),
                "position": { "type": "integer", "minimum": 1, "description": "Posisi berbasis 1 di antara anak induk (bawaan: terakhir)." },
                "top": { "type": "number", "minimum": -1000000, "maximum": 1000000, "description": "Poin dari atas halaman yang ditampilkan." },
                "left": { "type": "number", "minimum": -1000000, "maximum": 1000000, "description": "Poin dari kiri halaman yang ditampilkan." }
            }),
            &["doc", "title", "page"],
        )),
        t("bookmark_rename", "Ganti nama markah", "Ubah judul markah. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "path": path("Markah"), "title": { "type": "string", "minLength": 1 } }), &["doc", "path", "title"])),
        t("bookmark_delete", "Hapus markah", "Hapus markah dan markah di bawahnya. Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "path": path("Markah") }), &["doc", "path"])),
        t("bookmark_move", "Pindahkan markah", "Pindahkan markah (beserta anaknya) ke bawah induk lain, atau di dalam tingkatnya. Dapat dibatalkan.").with(schema(
            json!({ "doc": doc(), "path": path("Markah yang dipindahkan"), "parent": path("Induk baru (kosongkan untuk tingkat teratas)"), "position": { "type": "integer", "minimum": 1, "description": "Posisi berbasis 1 di antara anak induk baru, dihitung sesudah markah dihapus (bawaan: terakhir)." } }),
            &["doc", "path"],
        )),
        t("bookmark_set_page", "Atur halaman markah", "Arahkan markah ke halaman lain. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "path": path("Markah"), "page": { "type": "integer", "minimum": 1 } }), &["doc", "path", "page"])),
        t(
            "bookmark_set_style",
            "Atur gaya dan warna markah",
            "Warnai judul markah dan jadikan tebal atau miring (ISO 32000-2 §12.3.3: /C dan /F), seperti yang dilaporkan bookmark_list. Isi salah satu dari color ([r, g, b], masing masing 0-1; [0, 0, 0] adalah hitam bawaan), bold, dan italic; yang dikosongkan tetap seperti semula. Dapat dibatalkan.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "path": path("Markah"),
                "color": { "type": "array", "items": { "type": "number", "minimum": 0, "maximum": 1 }, "minItems": 3, "maxItems": 3 },
                "bold": { "type": "boolean" },
                "italic": { "type": "boolean" }
            }),
            &["doc", "path"],
        )),
        t(
            "bookmark_from_structure",
            "Markah baru dari struktur",
            "Buat markah dari heading bertag dokumen (H, H1-H6), bersarang menurut tingkat di bawah markah pertama baru berjudul \"Untitled\". Gagal bila dokumen tidak memiliki heading bertag. Mengembalikan halaman pertama pohon markah (lihat bookmark_list). Dapat dibatalkan.",
        )
        .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("page_number", "Beri nomor halaman", "Beri label rentang halaman (mis. i, ii, iii untuk bagian awal, atau A-1, A-2 untuk lampiran). Halaman sesudahnya mempertahankan labelnya. Dapat dibatalkan.").with(schema(
            json!({
                "doc": doc(),
                "from": { "type": "integer", "minimum": 1 },
                "to": { "type": "integer", "minimum": 1 },
                "style": { "type": "string", "enum": ["decimal", "upper-roman", "lower-roman", "upper-alpha", "lower-alpha", "none"], "description": "Gaya penomoran (bawaan decimal; none = awalan saja)." },
                "prefix": { "type": "string" },
                "start": { "type": "integer", "minimum": 1, "description": "Nomor halaman pertama dalam rentang (bawaan 1)." },
            }),
            &["doc", "from", "to"],
        )),
        t("form_fields", "Daftar bidang formulir", "Setiap bidang formulir interaktif: nama, jenis (text, checkbox, radio, combo, list, button, signature), nilai, opsi, halaman, rect (poin bertitik asal kiri atas), dan rotasi (0, 90, 180, atau 270 derajat berlawanan arah jarum jam), tanda baca saja dan wajib. Tombol tekan mencakup caption (label normal widget pertama) dan captions [{widget, caption}] untuk semua widget (berbasis 1).")
            .ro()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t(
            "form_fill",
            "Isi bidang formulir",
            "Atur beberapa bidang sekaligus (satu langkah batal). values memetakan nama bidang (dari form_fields) ke: string untuk bidang teks, kotak kombo, dan grup radio (satu opsi), true/false untuk kotak centang, array string untuk daftar pilihan ganda. Tampilan dibuat ulang sehingga setiap penampil menunjukkan nilainya. Nilai yang karakternya tidak dapat digambar font bidang (aksara Tionghoa dalam bidang Helvetica) tetap disimpan; `warnings` dalam hasil menamai bidang dan karakter: PdfCraft menggambar `?` di sana dan meminta penampil lain menggambar ulang bidang dari nilainya. Peringatan yang dimunculkan skrip formulir saat pengisian dikembalikan dalam `alerts`, dan yang dimintanya dari penampil (halaman, alamat web, pencetakan) dalam `requests`; nilai yang ditolak skrip bidang itu sendiri gagal dengan pesan skrip tersebut.",
        )
        .with(schema(json!({ "doc": doc(), "values": { "type": "object", "description": "Nama bidang → nilai." } }), &["doc", "values"])),
        t("form_reset", "Kosongkan formulir", "Atur ulang bidang ke nilai bawaannya: semua, atau hanya yang dicantumkan. Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "fields": { "type": "array", "items": { "type": "string" } } }), &["doc"])),
        t(
            "form_add_field",
            "Siapkan formulir: tambah bidang",
            "Tambah bidang formulir pada halaman. type: text, date, checkbox, radio, combo, list, button, image (penampung gambar; isi dengan form_set_image), signature. rect dalam poin dari kiri atas halaman yang ditampilkan [x0, y0, x1, y1]. name bawaan nama bebas berikutnya dari Acrobat (Text1, Check Box1, Group1, Dropdown1, List Box1, Button1, Image1, Signature1, Date1); nama bertitik seperti MASTER.first menjadikan bidang itu pertama di bawah bidang induk MASTER. Tombol radio bergabung ke grup radio yang dinamai group (grup baru bila tidak ada) dengan nilai ekspor export. Mengembalikan nama bidang. Dapat dibatalkan.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "page": { "type": "integer", "minimum": 1 },
                "type": { "type": "string", "enum": ["text", "date", "checkbox", "radio", "combo", "list", "button", "image", "signature"] },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                "name": { "type": "string" },
                "multiline": { "type": "boolean" },
                "group": { "type": "string", "description": "Grup radio tujuan bergabung." },
                "export": { "type": "string", "description": "Nilai ekspor tombol radio (bawaan Choice1)." },
                "options": { "type": "array", "items": { "type": "string" }, "description": "Item kotak kombo / kotak daftar." },
                "editable": { "type": "boolean", "description": "Kotak kombo menerima teks yang diketik." },
                "multi_select": { "type": "boolean" },
                "caption": { "type": "string", "description": "Label tombol." },
            }),
            &["doc", "page", "type", "rect"],
        )),
        t(
            "form_add_field_table",
            "Siapkan formulir: tambah tabel bidang teks",
            "Bagi rect menjadi baris dan kolom bidang teks, satu per sel, sebagai satu langkah batal. rect dalam poin dari kiri atas halaman yang ditampilkan [x0, y0, x1, y1]; rows 1-100 dari atas; columns 1-50, atau sebanyak yang dicantumkan names atau widths. Tiap kolom menamai selnya dari polanya dalam names: N tersendiri sesudah _ - / atau : (price_N), # atau {row} adalah nomor baris, {col} nomor kolom, {seq} menghitung tiap sel sepanjang baris, mulai dari first_row; bila tidak ada satu pun, nomor baris diletakkan di akhir. Baris dihitung dari first_row (bawaan 1). Tanpa names, kolom bernama Table1_N, Table1_c2_N, … memakai TableK bebas pertama. widths bersifat relatif (bawaan sama); row_gap dan column_gap adalah poin antar sel (bawaan 0). Tidak ada yang ditambah bila sel kurang dari 4 poin atau nama mengandung titik, berulang, atau sudah ada; galat menyatakan yang mana. Mengembalikan nama bidang. Dapat dibatalkan.",
        )
        .cmd("form.add.text_table")
        .with(schema(
            json!({
                "doc": doc(),
                "page": { "type": "integer", "minimum": 1 },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                "rows": { "type": "integer", "minimum": 1, "maximum": 100 },
                "columns": { "type": "integer", "minimum": 1, "maximum": 50 },
                "names": { "type": "array", "items": { "type": "string" }, "description": "Satu pola nama per kolom." },
                "widths": { "type": "array", "items": { "type": "number", "exclusiveMinimum": 0 }, "description": "Lebar kolom relatif, satu per kolom." },
                "row_gap": { "type": "number", "minimum": 0 },
                "column_gap": { "type": "number", "minimum": 0 },
                "first_row": { "type": "integer", "minimum": 0, "maximum": 99999 },
            }),
            &["doc", "page", "rect", "rows"],
        )),
        t("stamp_custom", "Tambah stempel khusus", "Stempel gambar: halaman PDF (file_page, bawaan 1, disimpan sebagai konten vektor dalam form XObject) atau berkas gambar. Baik terpusat pada at [x, y] (poin dari kiri atas halaman yang ditampilkan) pada ukuran aslinya, maksimal 200 pt, atau dipaskan di dalam rect [x0, y0, x1, y1] (koordinat yang sama; tiap sisi 1–14400 pt), diskalakan untuk menjaga bentuk dan terpusat dalam kotak. Seperti at, rect tidak dipotong ke halaman. Isi at atau rect, jangan keduanya. name melabelinya (bawaan: nama berkas). Dapat dibatalkan.")
            .cmd("comment.stamp")
            .with(schema(
                json!({
                    "doc": doc(),
                    "page": { "type": "integer", "minimum": 1 },
                    "path": { "type": "string" },
                    "at": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 },
                    "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                    "file_page": { "type": "integer", "minimum": 1 },
                    "name": { "type": "string" },
                    "author": { "type": "string" },
                }),
                &["doc", "page", "path"],
            )),
        t("form_set_image", "Atur gambar bidang gambar", "Tampilkan berkas gambar (PNG, JPEG, TIFF, GIF, atau BMP) dalam bidang gambar atau tombol, diskalakan agar pas dan terpusat (yang dilakukan saat bidang gambar diklik). Pada formulir XFA, gambar juga disimpan dalam datanya (base64, seperti yang disimpan Designer) dan formulir menampilkannya dari sana. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "field": { "type": "string" }, "path": { "type": "string" } }), &["doc", "field", "path"])),
        t(
            "form_set_props",
            "Properti bidang",
            "Ubah properti bidang: isi field untuk satu bidang, atau fields untuk daftar nama bidang unik (1–1000). Perubahan massal bersifat atomik dan satu langkah Undo; bidang yang tidak valid atau terkunci membuat semua bidang tidak berubah. Hanya properti yang diberikan yang berubah, dan perubahan tampilan sebagian mempertahankan warna dan font lain tiap bidang. name (ganti nama: nama bertitik seperti MASTER.first adalah nama lengkap dan memindahkan bidang ke bawah bidang induk tersebut; nama polos mengganti namanya di dalam induknya), rect (posisi), rotation, dan caption membutuhkan satu bidang. Penggantian nama juga mengganti nama bidang dalam perhitungan yang membacanya (daftar dan notasi bidang calculate, serta panggilan getField yang menamainya dalam literal string; nama yang dibangun saat runtime tidak ditulis ulang), dalam langkah Undo yang sama, dan formulas_updated mencantumkan bidang yang perhitungannya berubah. caption mengubah satu label normal tombol tekan (widget: indeks berbasis 1, bawaan 1); string kosong menghapusnya. Widget lain dan tata letak khusus ikon dipertahankan. General dan Options: tooltip, read_only, required, multiline, max_length (0 = tanpa batas), options (item combo/list), font_size (0 = otomatis), format, validate, calculate (aturan AF asli Acrobat). Options: align (left, center, right), default (nilai Reset form; kotak centang/tombol radio memakai status aktifnya, \"\" untuk nonaktif), flags { scroll, rich_text, password, file_select, spell_check, comb (membutuhkan max_length), sort, editable, multi_select, commit_immediately }, check_style (kotak centang/tombol radio: check, circle, cross, diamond, square, star). rotation adalah 0, 90, 180, atau 270 derajat berlawanan arah jarum jam dan menukar lebar/tinggi saat sumbu berubah. Dapat dibatalkan.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "field": { "type": "string", "description": "Nama bidang saat ini." },
                "fields": { "type": "array", "items": { "type": "string" }, "minItems": 1, "maxItems": 1000, "uniqueItems": true, "description": "Terapkan perubahan yang sama ke bidang ini dalam satu edit atomik. Gunakan field atau fields." },
                "name": { "type": "string" },
                "tooltip": { "type": "string" },
                "caption": { "type": "string", "description": "Label normal tombol tekan; string kosong menghapusnya. Membutuhkan satu bidang." },
                "widget": { "type": "integer", "minimum": 1, "description": "Widget yang caption-nya berubah (berbasis 1, bawaan 1). Membutuhkan caption. Lihat form_fields.captions." },
                "read_only": { "type": "boolean" },
                "required": { "type": "boolean" },
                "multiline": { "type": "boolean" },
                "max_length": { "type": "integer", "minimum": 0 },
                "options": { "type": "array", "items": { "type": "string" } },
                "align": { "type": "string", "enum": ["left", "center", "right"] },
                "locked": { "type": "boolean", "description": "Kunci properti bidang (hanya membuka kunci yang diterima saat terkunci)." },
                "default": { "type": "string" },
                "check_style": { "type": "string", "enum": ["check", "circle", "cross", "diamond", "square", "star"], "description": "Kotak centang dan tombol radio: tanda yang ditampilkan saat aktif." },
                "flags": { "type": "object", "additionalProperties": { "type": "boolean" } },
                "font_size": { "type": "number", "minimum": 0 },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4, "description": "Pindah/ubah ukuran: poin dari kiri atas halaman yang ditampilkan." },
                "rotation": { "type": "integer", "enum": [0, 90, 180, 270], "description": "Rotasi widget dalam derajat, berlawanan arah jarum jam. Perubahan antara horizontal dan vertikal menukar persegi di sekitar titik tengahnya." },
                "appearance": { "type": "object", "description": "Tab Appearance: border, fill, text_color (#RRGGBB, nama warna, atau \"none\"), width (0–12 poin; 1 tipis, 2 sedang, 3 tebal), style (solid, dashed, beveled, inset, underline), font (helvetica, times, courier)." },
                "format": { "description": "Tab Format: {\"type\": \"number\", \"decimals\": 2, \"currency\": \"$\", \"separator\": 0-4, \"negative\": 0-3}, {\"type\": \"percent\"}, {\"type\": \"date\"|\"time\", \"pattern\": \"mm/dd/yyyy\"}, {\"type\": \"zip\"|\"zip4\"|\"phone\"|\"ssn\"}, {\"type\": \"mask\", \"mask\": \"AA-9999\"}, atau \"none\"." },
                "validate": { "description": "Tab Validate: {\"min\": 0, \"max\": 100} (salah satu dapat dikosongkan) atau \"none\"." },
                "calculate": { "description": "Tab Calculate: {\"op\": \"sum\"|\"product\"|\"average\"|\"min\"|\"max\", \"fields\": [\"a\", \"b\"]}, {\"notation\": \"Price * Qty\"} (notasi bidang sederhana), atau \"none\"." },
            }),
            &["doc"],
        )),
        t("form_tab_order", "Atur urutan tab", "Atur urutan tab halaman (bawaan semua): row (atas ke bawah, kiri ke kanan), column, structure, atau annotations (tidak ditentukan). Atau urutkan tab secara manual: `field` dengan `move` earlier|later memindahkan bidang itu satu tempat pada halamannya. Mengembalikan urutan bidang hasil. Dapat dibatalkan.")
            .with(schema(
                json!({ "doc": doc(), "order": { "type": "string", "enum": ["row", "column", "structure", "annotations"] }, "pages": pages("yang diatur (bawaan: semua)"), "field": { "type": "string" }, "move": { "type": "string", "enum": ["earlier", "later"] } }),
                &["doc"],
            )),
        t("form_calculation_order", "Baca atau atur urutan perhitungan", "Bidang terhitung dalam urutan perhitungannya (Acrobat: Set Field Calculation Order), masing masing dengan bidang yang dibaca rumusnya (`reads`; nama skrip yang dibangun saat runtime tidak diketahui) dan `issues`-nya: runs_too_early (membaca `field`, yang menghitung sesudahnya, sehingga menampilkan nilai basi), cycle (membaca `fields`, yang membacanya kembali: tidak ada urutan yang memperbaikinya, rumus harus diubah) dan not_in_order (urutan PDF tidak memasukkannya, sehingga penampil lain mungkin tidak menghitungnya). `extra` mencantumkan entri urutan PDF yang tidak menghitung apa pun. Tanpa perubahan, hanya membaca. Perubahan, satu dalam satu waktu: `order` (bidang ini lebih dulu, dalam urutan ini, lalu bidang terhitung lain dalam urutannya), `field` dengan `move` earlier|later (satu tempat), atau `fix` true (hanya yang harus pindah yang pindah, sehingga tiap bidang menghitung sesudah bidang yang dibacanya, dan entri extra hilang). Perubahan menghitung ulang formulir. Dapat dibatalkan.")
            .cmd("form.calculation_order")
            .with(schema(
                json!({ "doc": doc(), "order": { "type": "array", "items": { "type": "string" } }, "field": { "type": "string" }, "move": { "type": "string", "enum": ["earlier", "later"] }, "fix": { "type": "boolean" } }),
                &["doc"],
            )),
        t("doc_export_data", "Ekspor komentar atau data formulir", "Tulis komentar dan/atau data formulir ke path; ekstensi memilih format: .xfdf atau .fdf (komentar dan/atau bidang), .xml, .csv, atau .txt (data formulir). Untuk formulir XFA, .xml adalah datanya saja (berakar pada node data akar formulir, seperti yang diekspor Acrobat) dan .xdp adalah XML Data Package yang memuat seluruh datasets. what: all (bawaan), comments, fields.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" }, "what": { "type": "string", "enum": ["all", "comments", "fields"] } }), &["doc", "path"])),
        t("doc_import_data", "Impor komentar atau data formulir", "Impor komentar dan/atau nilai bidang dari berkas XFDF, FDF, XML, CSV, atau teks terpisah tab (dideteksi dari kontennya). Komentar dengan nama sama diganti; nilai melewati format dan validasi formulir. Untuk formulir XFA, berkas XDP, datasets, atau XML data mengganti data formulir: formulir yang tertata ditata ulang darinya (termasuk baris) dan perhitungannya dijalankan. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" } }), &["doc", "path"])),
        t("form_delete_field", "Hapus bidang", "Hapus bidang formulir dan semua widgetnya. Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "field": { "type": "string" } }), &["doc", "field"])),
        t(
            "redact_mark",
            "Tandai untuk redaksi",
            "Tandai konten untuk redaksi (tidak ada yang dihapus sampai redact_apply). Salah satu dari: rect [x0, y0, x1, y1] (poin dari kiri atas halaman yang ditampilkan) dengan page; find (teks, setiap kecocokan); words (daftar kata atau frasa, setiap kecocokan masing masing; matched_words dalam hasil menyatakan berapa tanda yang dibuat masing masing); pattern (phone, email, credit-card, ssn, date: setiap kecocokan, pola Search & Redact Acrobat); whole_pages: true. find dan words menerima case_sensitive dan whole_words (keduanya bawaan false). Pilih satu halaman dengan page atau banyak dengan pages (bawaan semua); jangan isi keduanya. rect membutuhkan page. overlay: teks yang ditampilkan pada kotak sesudah diterapkan, atau code_set (foia: U.S. FOIA (b)(1)(A)…(b)(9); privacy-act: U.S. Privacy Act (d)(5), (j)(1)…(k)(7)) dengan codes, ditampilkan sebagai \"(b)(6), (b)(7)(C)\"; fill: warna kotak (bawaan hitam). Dapat dibatalkan.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "page": { "type": "integer", "minimum": 1, "description": "Satu halaman yang ditandai; saling eksklusif dengan pages." },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                "find": { "type": "string", "minLength": 1 },
                "words": { "type": "array", "items": { "type": "string", "minLength": 1 }, "minItems": 1, "maxItems": 1000 },
                "case_sensitive": { "type": "boolean", "description": "Cocokkan huruf besar kecil untuk find atau words (bawaan false)." },
                "whole_words": { "type": "boolean", "description": "Cocokkan kata utuh untuk find atau words (bawaan false)." },
                "pattern": { "type": "string", "enum": ["phone", "email", "credit-card", "ssn", "date"] },
                "whole_pages": { "type": "boolean" },
                "pages": pages("yang dicari atau ditandai (bawaan: semua); saling eksklusif dengan page, dan tidak valid dengan rect"),
                "overlay": { "type": "string" },
                "code_set": { "type": "string", "enum": ["foia", "privacy-act"] },
                "codes": { "type": "array", "items": { "type": "string" }, "minItems": 1, "description": "Kode redaksi dari code_set, mis. [\"(b)(6)\"]." },
                "fill": { "type": "string", "description": "#RRGGBB atau nama warna." },
                "author": { "type": "string" },
            }),
            &["doc"],
        )),
        t(
            "redact_apply",
            "Terapkan redaksi",
            "Terapkan tanda redaksi (semua, atau yang pada pages): teks, gambar, vektor, komentar, dan bidang formulir di bawahnya dihapus permanen dan kotak digambar sebagai gantinya. Tahap verifikasi menggagalkan operasi bila masih ada yang dapat dibaca. Dapat dibatalkan sampai disimpan; berkas yang disimpan tidak lagi memuat konten tersebut.",
        )
        .destructive()
        .with(schema(json!({ "doc": doc(), "pages": pages("yang tandanya diterapkan (bawaan: semua)") }), &["doc"])),
        t("redact_clear", "Hapus tanda redaksi", "Hapus setiap tanda redaksi tanpa menerapkannya. Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t(
            "doc_hidden_info",
            "Temukan informasi tersembunyi",
            "Yang akan dihapus Remove Hidden Information, menurut kategori (metadata, attachments, comments, form-fields, hidden-text, hidden-layers, bookmarks, links-actions-scripts, private-data) beserta jumlahnya.",
        )
        .ro()
        .with(schema(json!({ "doc": doc() }), &["doc"])),
        t(
            "doc_remove_hidden",
            "Hapus informasi tersembunyi / Bersihkan",
            "Hapus kategori yang dicantumkan (lihat doc_hidden_info), atau setiap kategori bila tidak ada yang dicantumkan (Sanitize Document). Bidang formulir diratakan agar nilainya tetap terlihat. Penyimpanan berikutnya menulis ulang seluruh berkas. Dapat dibatalkan sampai disimpan.",
        )
        .destructive()
        .with(schema(
            json!({
                "doc": doc(),
                "categories": { "type": "array", "items": { "type": "string", "enum": ["metadata", "attachments", "comments", "form-fields", "hidden-text", "hidden-layers", "bookmarks", "links-actions-scripts", "private-data"] } },
            }),
            &["doc"],
        )),
        t(
            "doc_audit_redactions",
            "Audit redaksi palsu",
            "Temukan redaksi palsu: persegi terisi buram yang digambar di atas teks yang masih dapat diekstrak, dan anotasi /Redact yang ditandai tetapi tidak pernah diterapkan. Memperhatikan urutan gambar dan warna: persegi hanya terhitung bila digambar sesudah teks dan cukup gelap untuk menyembunyikannya (latar halaman dan arsiran terang tidak ditandai). Tiap temuan melaporkan halaman berbasis 1, jenis (covered-text atau unapplied-mark), persegi (poin dari kiri atas halaman yang ditampilkan, seperti comment_list), dan teks tersembunyi. Gagal tertutup: konten yang tidak dapat dibaca adalah galat, tidak pernah dinyatakan bersih. Redaksi yang diterapkan dengan benar menghapus teks dan tidak pernah ditandai.",
        )
        .ro()
        .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("printers", "Daftar printer", "Printer yang dikenal spooler cetak sistem (CUPS pada macOS dan Linux), dengan bawaan ditandai.").ro().with(schema(json!({}), &[])),
        t("printer_options", "Daftar opsi printer", "Opsi tugas milik driver printer (CUPS: dari PPD-nya), seperti baki kertas, jenis kertas, atau penyelesaian: key, label, group, default, dan choices. Isi nilai terpilih ke doc_print sebagai options. Kosong pada Windows, tempat jendela preferensi driver menyimpannya.")
            .ro()
            .with(schema(json!({ "printer": { "type": "string" } }), &["printer"])),
        t(
            "doc_print",
            "Cetak",
            "Cetak dengan opsi dialog Print Acrobat, atau simpan PDF siap cetak. pages: rentang seperti \"1-3, 6, 9-\" (label halaman diperbolehkan; bawaan semua); subset odd/even (menurut nomor halaman, seperti yang dihitung Acrobat: pages 2, 7-10 dengan even mencetak 2, 8, dan 10, bukan setiap halaman kedua dari rentang); reverse. layout: fit (bawaan), actual, shrink, custom (scale %), multiple (per_sheet 2/4/6/9/16, order, border, auto_rotate; urutan cut-stack menata lembar satu sisi untuk dipotong menjadi tumpukan dan disusun kiri ke kanan, atas ke bawah, menjaga urutan lembar dalam tiap tumpukan; duplex harus off), booklet (booklet_subset both/front/back, binding left/right; booklet mengatur dua sisi sendiri, seperti Acrobat: both membalik pada tepi pendek, front atau back mencetak satu sisi), poster (scale %, overlap pt, cut_marks). orientation auto/portrait/landscape; comments_forms document / document-and-markups (bawaan) / document-and-stamps / form-fields-only; paper Letter/Legal/Tabloid, ARCH A/B/C/D/E1/E, ANSI C/D/E, A0/A1/A2/A3/A4/A5/A6, atau US #10 Envelope (nama pendek atau label preset lengkap); gunakan layout actual untuk 100% dalam PDF siap cetak; driver printer mungkin tetap mengganti kertas atau skala. Lalu path (simpan) atau printer (nama atau \"default\") dengan copies, collate, duplex off/long-edge/short-edge, grayscale, dan options (opsi driver dari printer_options, CUPS).",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "pages": { "type": "string" },
                "subset": { "type": "string", "enum": ["all", "odd", "even"] },
                "reverse": { "type": "boolean" },
                "layout": { "type": "string", "enum": ["fit", "actual", "shrink", "custom", "multiple", "booklet", "poster"] },
                "scale": { "type": "number", "exclusiveMinimum": 0 },
                "per_sheet": { "type": "integer", "minimum": 1, "maximum": 256 },
                "order": { "type": "string", "enum": ["horizontal", "horizontal-reversed", "vertical", "vertical-reversed", "cut-stack"] },
                "border": { "type": "boolean" },
                "auto_rotate": { "type": "boolean" },
                "booklet_subset": { "type": "string", "enum": ["both", "front", "back"] },
                "binding": { "type": "string", "enum": ["left", "right"] },
                "overlap": { "type": "number", "minimum": 0 },
                "cut_marks": { "type": "boolean" },
                "orientation": { "type": "string", "enum": ["auto", "portrait", "landscape"] },
                "comments_forms": { "type": "string", "enum": ["document", "document-and-markups", "document-and-stamps", "form-fields-only"] },
                "paper": { "type": "string" },
                "path": { "type": "string" },
                "printer": { "type": "string" },
                "copies": { "type": "integer", "minimum": 1, "maximum": 999 },
                "collate": { "type": "boolean" },
                "duplex": { "type": "string", "enum": ["off", "long-edge", "short-edge", "printer"], "description": "Pencetakan dua sisi (bawaan off); printer tidak mengirim pengaturan, sehingga bawaan printer berlaku." },
                "grayscale": { "type": "boolean" },
                "as_image": { "type": "boolean", "description": "Cetak sebagai gambar (Advanced Print Setup Acrobat): tiap halaman dirender menjadi gambar pada dpi sebelum diletakkan di kertas, untuk printer yang melewatkan teks, arsiran, atau transparansi; hasil cetak tidak lagi berupa teks yang dapat dicari, dan dengan grayscale gambar menjadi abu abu." },
                "dpi": { "type": "number", "minimum": 72, "maximum": 600, "description": "Resolusi untuk as_image (bawaan 300)." },
                "options": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Opsi driver printer dari printer_options: key → nilai pilihan." },
            }),
            &["doc"],
        )),
        t("content_list", "Daftar konten tambahan", "Teks dan gambar yang ditambah dengan page_add_text/page_add_image (atau Edit a PDF ▸ Add content), per halaman dengan indeks berbasis 1, rect (poin dari kiri atas halaman), dan gaya teks.")
            .ro()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t(
            "page_add_text",
            "Tambah teks ke halaman",
            "Tambah teks sebagai konten halaman (bukan komentar). Tempatkan dengan at [x, y] (kiri atas, poin dari kiri atas halaman yang ditampilkan) dan width (lebar bungkus, bawaan 200), atau rect. Baris baru memulai baris baru; baris panjang dibungkus. Gaya: font helvetica/times/courier, size, bold, italic, color (#RRGGBB atau nama warna), align left/center/right, char_spacing (poin sesudah tiap karakter). Teks harus berupa karakter Eropa Barat (WinAnsi), atau Arab, Ibrani, atau Thai bila build memiliki face craft-fonts-nya (digambar kanan ke kiri pada teksnya); yang lain ditolak. Tetap dapat diedit dengan content_update. Dapat dibatalkan.",
        )
        .cmd("edit.text")
        .with(schema(
            json!({
                "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "text": { "type": "string", "minLength": 1 },
                "at": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 },
                "width": { "type": "number", "exclusiveMinimum": 0 },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                "font": { "type": "string", "enum": ["helvetica", "times", "courier"] }, "size": { "type": "number", "minimum": 1, "maximum": 500 },
                "bold": { "type": "boolean" }, "italic": { "type": "boolean" }, "color": { "type": "string" },
                "align": { "type": "string", "enum": ["left", "center", "right", "justify"] },
                "char_spacing": { "type": "number", "minimum": -500, "maximum": 500, "description": "Poin sesudah tiap karakter." },
            }),
            &["doc", "page", "text"],
        )),
        t(
            "page_add_image",
            "Tambah gambar ke halaman",
            "Tambah berkas gambar (PNG, JPEG, TIFF, GIF, BMP) sebagai konten halaman: dalam rect (poin dari kiri atas halaman), atau terpusat pada ukuran aslinya (diciutkan agar pas). Dapat dibatalkan; dapat dipindahkan dengan content_update.",
        )
        .cmd("edit.image")
        .with(schema(
            json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "path": { "type": "string" }, "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 } }),
            &["doc", "page", "path"],
        )),
        t("content_update", "Edit konten tambahan", "Pindah/ubah ukuran (rect), ketik ulang (text), atau ganti gaya (font, size, bold, italic, color, align, char_spacing dalam poin) item tambahan (page, index dari content_list). Gambar: rotate (derajat, kelipatan 90, berlawanan arah jarum jam), flip_h / flip_v (alih), crop [left, bottom, right, top] sebagai pecahan yang dipangkas, image (berkas yang mengganti gambar). Dapat dibatalkan.")
            .with(schema(
                json!({
                    "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "index": { "type": "integer", "minimum": 1 },
                    "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 }, "text": { "type": "string" },
                    "font": { "type": "string", "enum": ["helvetica", "times", "courier"] }, "size": { "type": "number", "minimum": 1, "maximum": 500 },
                    "bold": { "type": "boolean" }, "italic": { "type": "boolean" }, "color": { "type": "string" },
                    "align": { "type": "string", "enum": ["left", "center", "right", "justify"] },
                    "char_spacing": { "type": "number", "minimum": -500, "maximum": 500, "description": "Points after each character." },
                    "rotate": { "type": "integer" }, "flip_h": { "type": "boolean" }, "flip_v": { "type": "boolean" },
                    "crop": { "type": "array", "items": { "type": "number", "minimum": 0, "maximum": 0.49 }, "minItems": 4, "maxItems": 4 },
                    "image": { "type": "string", "description": "Ganti gambar dengan berkas ini." },
                }),
                &["doc", "page", "index"],
            )),
        t(
            "content_copy",
            "Salin dan tempel konten tambahan",
            "Salin item tambahan (page, index dari content_list) dan tempelkan, seperti Edit ▸ Copy dan Paste: ke to_page dari to_doc (bawaan halaman dan dokumennya sendiri) apa adanya: teks, font, ukuran, warna, perataan, dan kotak kotak teks; gambar, kotak, putaran, balik, dan pangkas gambar (di dalam dokumennya objek gambar yang sama digambar lagi; ke dokumen lain gambar masuk sebagai berkas JPEG atau JPEG 2000-nya, atau PNG bila tidak, tanpa profil warna dan dengan warna CMYK diubah ke RGB). Ditempatkan pada at [x, y] (kiri atasnya, poin dari kiri atas halaman), atau tempat semula, 18 poin ke bawah dan ke kanan melewati tiap salinannya yang sudah ada; dijaga tetap pada halaman. Ditolak bila dokumen sumber tidak mengizinkan penyalinan. Mengembalikan page, index, dan rect item baru. Dapat dibatalkan pada dokumen tujuan.",
        )
        .with(schema(
            json!({
                "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "index": { "type": "integer", "minimum": 1 },
                "to_doc": { "type": "integer", "minimum": 1, "description": "Dokumen tujuan tempel (bawaan doc)." },
                "to_page": { "type": "integer", "minimum": 1, "description": "Halaman tujuan tempel (bawaan page)." },
                "at": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 },
            }),
            &["doc", "page", "index"],
        )),
        t("content_delete", "Hapus konten tambahan", "Hapus item tambahan (page, index dari content_list). Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "index": { "type": "integer", "minimum": 1 } }), &["doc", "page", "index"])),
        t("link_list", "Daftar tautan", "Setiap tautan: page, indeks berbasis 1 (untuk link_edit/link_delete), rect (poin dari kiri atas halaman), dan tujuannya (url atau to_page).")
            .ro()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("link_open", "Buka PDF tertaut", "Ikuti tautan GoToR lokal dalam PdfCraft: buka PDF yang dirujuk sebagai dokumen baru, dengan memperhatikan --root. page dan index memakai penomoran link_list. Tujuan bernama dilaporkan tetapi tidak diterapkan. Spesifikasi berkas absolut membutuhkan allow_absolute: true sesudah memeriksa tujuan; tautan relatif tidak membutuhkan persetujuan.")
            .with(schema(
                json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "index": { "type": "integer", "minimum": 1 }, "password": { "type": "string", "description": "Kata sandi opsional untuk PDF tertaut." }, "allow_absolute": { "type": "boolean", "description": "Izinkan secara eksplisit tujuan GoToR lokal absolut (bawaan false). Validasi yang ada dan pembatasan --root tetap berlaku." } }),
                &["doc", "page", "index"],
            )),
        t("link_add", "Tambah tautan", "Tambah tautan di atas rect pada page yang membuka url atau menuju to_page. Tampilan: visible (persegi), color, width 1-3, highlight none/invert/outline/inset. Dapat dibatalkan.")
            .cmd("edit.link")
            .with(schema(
                json!({
                    "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                    "url": { "type": "string" }, "to_page": { "type": "integer", "minimum": 1 },
                    "visible": { "type": "boolean" }, "color": { "type": "string" }, "width": { "type": "number" }, "highlight": { "type": "string", "enum": ["none", "invert", "outline", "inset"] },
                }),
                &["doc", "page", "rect"],
            )),
        t("link_edit", "Edit tautan", "Ubah rect, tujuan (url atau to_page), atau tampilan tautan. Dapat dibatalkan.")
            .with(schema(
                json!({
                    "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "index": { "type": "integer", "minimum": 1 },
                    "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                    "url": { "type": "string" }, "to_page": { "type": "integer", "minimum": 1 },
                    "visible": { "type": "boolean" }, "color": { "type": "string" }, "width": { "type": "number" }, "highlight": { "type": "string", "enum": ["none", "invert", "outline", "inset"] },
                }),
                &["doc", "page", "index"],
            )),
        t("link_delete", "Hapus tautan", "Hapus satu tautan (page, index dari link_list). Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "index": { "type": "integer", "minimum": 1 } }), &["doc", "page", "index"])),
        t("links_from_urls", "Buat tautan dari URL", "Temukan alamat web (http://, https://, www.) dalam teks tiap halaman dan jadikan tautan yang dapat diklik. Mengembalikan URL. Dapat dibatalkan.")
            .cmd("edit.links_from_urls")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("links_remove", "Hapus semua tautan", "Hapus setiap tautan dalam dokumen (atau pada halaman tertentu). Dapat dibatalkan.")
            .destructive()
            .cmd("edit.remove_links")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang tautannya dihapus (bawaan: semua)") }), &["doc"])),
        t("measure_distance", "Ukur jarak", "Tambah anotasi jarak dua titik yang dapat dibatalkan memakai skala viewport titik pertama.")
            .cmd("measure.distance").with(measure_schema()),
        t("measure_perimeter", "Ukur keliling", "Tambah anotasi panjang garis tersambung yang dapat dibatalkan. Untuk menyertakan tepi penutup, ulangi titik pertama di akhir.")
            .cmd("measure.perimeter").with(measure_schema()),
        t("measure_area", "Ukur luas", "Tambah anotasi luas yang dapat dibatalkan dari poligon sederhana. Tepi terakhir menutup otomatis.")
            .cmd("measure.area").with(measure_schema()),
        t("measure_info", "Baca pengukuran", "Hitung jarak, keliling, atau luas secara langsung, beserta delta, sudut, dan skala tanpa menambah anotasi. Jalur yang tidak lengkap diperbolehkan.")
            .ro().cmd("measure.info").with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1},"points":measure_points(),"type":{"type":"string","enum":["distance","perimeter","area"]}}), &["doc","page","points"])),
        t("measure_list", "Daftar pengukuran", "Anotasi pengukuran tersimpan dengan nilai terhitung, skala, dan simpul dalam koordinat tampilan. Pengukuran dengan format impor yang tidak didukung (satuan majemuk atau pecahan, skala non rektilinear) atau geometri tidak valid dicantumkan di bawah unsupported beserta alasan.")
            .ro().with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1}}), &["doc"])),
        t("measure_scale", "Atur atau baca skala pengukuran", "Baca skala pada satu titik, atau tambah viewport persegi memakai rasio (page_distance dalam page_unit pada halaman = distance dalam unit, mis. 1 mm = 50 mm untuk denah 1:50; page_unit bawaan unit), units_per_point, atau dua titik kalibrasi dan jarak nyata keduanya. Pengukuran yang ada mempertahankan skala aslinya. Dapat dibatalkan.")
            .cmd("measure.scale").with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1},"at":point(),"rect":{"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4},"name":{"type":"string"},"unit":{"type":"string"},"precision":{"type":"integer","minimum":0,"maximum":6},"units_per_point":{"type":"number","exclusiveMinimum":0},"page_distance":{"type":"number","exclusiveMinimum":0},"page_unit":{"type":"string","enum":["pt","in","mm","cm"]},"points":measure_points(),"distance":{"type":"number","exclusiveMinimum":0}}), &["doc","page"])),
        t("measure_snap", "Kaitkan simpul pengukuran", "Kaitkan titik ke jalur vektor, titik akhir, titik tengah, atau perpotongan. Koordinat dan toleransi dalam poin tampilan. Ekstraksi terbatas melaporkan geometri yang terpotong.")
            .ro().cmd("measure.snap").with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1},"at":point(),"tolerance":{"type":"number","minimum":0,"maximum":10000},"endpoints":{"type":"boolean"},"midpoints":{"type":"boolean"},"intersections":{"type":"boolean"},"paths":{"type":"boolean"}}), &["doc","page","at"])),
        t("measure_export", "Ekspor pengukuran sebagai CSV", "Tulis secara atomik nilai pengukuran tersimpan, label, penulis, dan rasio skala sebagai CSV yang aman untuk spreadsheet. Mengembalikan berapa pengukuran tidak didukung yang ditinggalkan.")
            .cmd("measure.export").with(schema(json!({"doc":doc(),"out":path_arg()}), &["doc","out"])),
        t("comment_list", "Daftar komentar", "Setiap komentar (anotasi selain tautan, widget formulir, dan pop up) beserta halaman, indeks, id, jenis, penulis, teks, tanggal, persegi, warna, status tinjauan, tanda centang, dan balasannya (masing masing dengan status dan tanda centang sendiri).")
            .ro()
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1, "description": "Hanya halaman ini." } }), &["doc"])),
        t(
            "comment_add",
            "Tambah komentar",
            "Tambah komentar seperti yang dilakukan alat komentar Acrobat. Geometri dalam poin dengan titik asal pada kiri atas halaman yang ditampilkan, y ke bawah (seperti pada gambar page_render pada 72 dpi dan rect text_find). \
             note: `at` [x, y] (kiri atas ikon). stamp: `at` [x, y] (titik tengahnya) dan `stamp`: approved, completed, confidential, draft, final, for comment, for public release, information only, not approved, not for public release, preliminary results, void, accepted, initial here, rejected, sign here, witness; dynamic: true untuk stempel approved/confidential/received/reviewed/revised dinamis dengan baris By … at …. highlight/underline/strikeout/squiggly: `find` (teks pada halaman yang ditandai; setiap kecocokan dengan all: true) atau `quads`. \
             rectangle/oval/textbox: `rect` [x0, y0, x1, y1]. line/arrow: `from`, `to`. `endings` opsional (dua nama: awal, akhir) untuk garis, panah, atau polyline, atau satu nama untuk callout: None, Square, Circle, Diamond, OpenArrow, ClosedArrow, Butt, ROpenArrow, RClosedArrow, Slash. Panah tanpa endings adalah None lalu OpenArrow. ink: `strokes` [[[x, y], …], …]. \
             polygon/cloud/polyline (garis tersambung): `points` [[x, y], …]. callout: `rect` (kotak teksnya), `to` (titik yang disentuh panah), `knee` opsional. caret (teks sisipan): `at`, titik penyisipan pada garis dasar. replace (Replace Text): `find` atau `quads` seperti highlight, `contents` penggantinya. attachment: `path` (berkas), `at`, `icon` opsional (PushPin, Paperclip, Graph, Tag). sound (komentar suara): `path` (rekaman .wav PCM 16-bit, mono atau stereo), `at`. Dapat dibatalkan. Teks yang digambar kotak teks, callout, atau stempel dalam font standar yang tidak dapat digambarnya (Tionghoa, Yunani) disimpan seperti diketik dan digambar sebagai `?`; `warnings` dalam hasil menyatakannya.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "page": { "type": "integer", "minimum": 1 },
                "type": { "type": "string", "enum": ["note", "highlight", "underline", "strikeout", "squiggly", "replace", "rectangle", "oval", "line", "arrow", "ink", "textbox", "stamp", "polygon", "cloud", "polyline", "callout", "caret", "attachment", "sound"] },
                "path": { "type": "string", "description": "attachment: berkas yang dilampirkan; sound: rekaman .wav." },
                "points": { "type": "array", "items": point(), "minItems": 2 },
                "knee": point(),
                "contents": { "type": "string", "description": "Teks komentar (yang ditampilkan kotak teks)." },
                "author": { "type": "string" },
                "at": point(),
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                "from": point(),
                "to": point(),
                "find": { "type": "string", "description": "Teks pada halaman yang ditandai (tidak peka huruf besar kecil)." },
                "all": { "type": "boolean", "description": "Tandai setiap kecocokan `find` pada halaman, bukan hanya yang pertama." },
                "quads": { "type": "array", "items": { "type": "array", "items": { "type": "number" }, "minItems": 8, "maxItems": 8 } },
                "stamp": { "type": "string" },
                "dynamic": { "type": "boolean" },
                "strokes": { "type": "array", "items": { "type": "array", "items": point() } },
                "icon": { "type": "string", "description": "note: Comment, Note, Help, Insert, Key, NewParagraph, Paragraph (nama standar), Check, Circle, Cross, CrossHairs, Star, RightArrow, UpArrow; attachment: PushPin, Paperclip, Graph, Tag." },
                "color": color(),
                "fill": color(),
                "opacity": { "type": "number", "minimum": 0, "maximum": 1 },
                "width": { "type": "number", "minimum": 0, "description": "Lebar garis dalam poin." },
                "endings": { "type": "array", "items": { "type": "string", "enum": ["None", "Square", "Circle", "Diamond", "OpenArrow", "ClosedArrow", "Butt", "ROpenArrow", "RClosedArrow", "Slash"] }, "minItems": 1, "maxItems": 2, "description": "Ujung garis. Dua nama (awal, akhir) untuk garis, panah, atau polyline; satu nama untuk callout." },
                "font_size": { "type": "number", "exclusiveMinimum": 0 },
            }),
            &["doc", "page", "type"],
        )),
        t("comment_reply", "Balas komentar", "Tambah balasan ke utas komentar. Dapat dibatalkan.").with(schema(
            comment_ref(json!({ "text": { "type": "string", "minLength": 1 }, "author": { "type": "string" } })),
            &["doc", "text"],
        )),
        t("comment_set_status", "Atur status komentar", "Atur status tinjauan komentar (Acrobat: Set status), dicatat sebagai balasan status. Dapat dibatalkan.").with(schema(
            comment_ref(json!({ "status": { "type": "string", "enum": ["none", "accepted", "rejected", "cancelled", "completed"] }, "author": { "type": "string" } })),
            &["doc", "status"],
        )),
        t("sign_list", "Daftar dan validasi tanda tangan", "Setiap bidang tanda tangan beserta validasinya: status (valid, unknown = utuh tetapi penanda tangan tidak dipercaya, invalid, unsigned), penanda tangan dan sertifikat, tanggal, alasan, lokasi, tingkat sertifikasi, halaman dan rect, revisi yang dicakupnya dan perubahan yang dibuat sesudah penandatanganan (none, allowed, disallowed), dengan penjelasan gaya Acrobat.")
            .ro()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("sign_id_create", "Buat ID digital", "Buat ID digital yang ditandatangani sendiri dan simpan sebagai berkas .p12 yang dilindungi kata sandi (Acrobat: Configure a new digital ID ▸ Create a new digital ID ▸ Save to file). Kunci bawaan: RSA 2048-bit, berlaku 5 tahun.").with(schema(
            json!({
                "name": { "type": "string", "minLength": 1 },
                "organization": { "type": "string" },
                "unit": { "type": "string" },
                "email": { "type": "string" },
                "country": { "type": "string", "description": "Kode negara dua huruf." },
                "key": { "type": "string", "enum": ["rsa2048", "rsa3072", "rsa4096", "p256"] },
                "years": { "type": "integer", "minimum": 1, "maximum": 50 },
                "password": { "type": "string", "minLength": pdfcraft_engine::sign::pkcs12::MIN_PASSWORD_CHARS },
                "path": { "type": "string", "description": "Berkas .p12 tujuan penulisan." },
            }),
            &["name", "password", "path"],
        )),
        t("sign_windows_ids", "Daftar ID digital penyimpanan Windows", "Windows: identitas penandatanganan dalam penyimpanan sertifikat Current User Personal (rincian sertifikat dan referensi windows: yang diterima sign_document). Kunci privat tetap dalam CNG; Windows mungkin meminta izin untuk memakainya. `unusable` mencantumkan sertifikat lain penyimpanan beserta alasan tidak dapat menandatangani (tidak ada kunci privat, jenis kunci tidak didukung, kunci yang tidak dapat dibuka CNG).")
            .ro()
            .cmd("sign.digital")
            .with(schema(json!({}), &[])),
        t("sign_keychain_ids", "Daftar ID digital Keychain", "macOS: identitas penandatanganan dalam keychain pengguna (rincian sertifikat dan referensi keychain: yang diterima sign_document). Kunci privat tetap dalam Keychain, yang mungkin meminta izin pengguna untuk memakainya.")
            .ro()
            .cmd("sign.digital")
            .with(schema(json!({}), &[])),
        t("sign_document", "Tanda tangani dokumen", "Tanda tangani dengan ID digital (path .p12/.pfx, atau pada macOS identitas Keychain: \"keychain:<common name or fingerprint>\" dari sign_keychain_ids, atau pada Windows identitas penyimpanan: \"windows:<common name or fingerprint>\" dari sign_windows_ids) dan simpan berkas yang ditandatangani ke `out` (penandatanganan selalu menyimpan, seperti dalam Acrobat; dokumen lalu menampilkan berkas yang ditandatangani). Tanda tangani bidang tanda tangan kosong yang ada (`field`), atau bidang baru pada `page` pada `rect` (kosongkan rect untuk tanda tangan tidak terlihat). certify: no_changes, form_fill, atau comments membuat tanda tangan sertifikasi. graphic menampilkan gambar tempat nama penanda tangan akan besar (Acrobat: Configure Signature Appearance): {\"image\": path} (PNG atau JPEG), {\"text\": name} (diketik dalam font script Fill & Sign), atau {\"strokes\": [[[x, y], …], …]} (y ke atas, bantalan gambar selebar 1); membutuhkan tanda tangan yang terlihat. appearance memilih yang ditampilkan tanda tangan yang terlihat (bawaan: name, date, dan labels; reason, location, dan distinguished_name nonaktif kecuali diaktifkan). labels memberikan kata kata labelnya, mis. dalam bahasa lain (bawaan Bahasa Inggris: \"Digitally signed by\", \"DN: \", \"Reason: \", \"Location: \", \"Date: \"; sertakan pemisah); masing masing harus dalam karakter Eropa Barat (Windows-1252), satu satunya yang digambar font tanda tangan, atau panggilan ditolak dengan menamai label dan karakter. PAdES B-B, SHA-256 (SHA-384 untuk kunci P-384).")
            .cmd("sign.digital")
            .with(schema(
                json!({
                    "doc": doc(),
                    "id": { "type": "string", "description": "Berkas ID digital (.p12 / .pfx), referensi keychain: pada macOS, atau referensi windows: pada Windows." },
                    "password": { "type": "string" },
                    "field": { "type": "string" },
                    "page": { "type": "integer", "minimum": 1 },
                    "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                    "reason": { "type": "string" },
                    "location": { "type": "string" },
                    "contact": { "type": "string" },
                    "certify": { "type": "string", "enum": ["no_changes", "form_fill", "comments"] },
                    "graphic": {
                        "type": "object",
                        "description": "Tepat satu dari image, text, atau strokes.",
                        "properties": {
                            "image": { "type": "string", "description": "Berkas PNG atau JPEG." },
                            "text": { "type": "string", "description": "Diketik dalam font script." },
                            "strokes": { "type": "array", "items": { "type": "array", "items": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 } } },
                        },
                        "minProperties": 1,
                        "maxProperties": 1,
                    },
                    "appearance": {
                        "type": "object",
                        "description": "Yang ditampilkan tanda tangan yang terlihat; item yang tidak diberikan mempertahankan bawaannya.",
                        "properties": {
                            "name": { "type": "boolean", "description": "Bawaan true." },
                            "date": { "type": "boolean", "description": "Bawaan true." },
                            "reason": { "type": "boolean", "description": "Bawaan false." },
                            "location": { "type": "boolean", "description": "Bawaan false." },
                            "distinguished_name": { "type": "boolean", "description": "Bawaan false." },
                            "labels": { "type": "boolean", "description": "Bawaan true." },
                        },
                        "additionalProperties": false,
                    },
                    "labels": {
                        "type": "object",
                        "description": "Kata kata label tanda tangan yang terlihat (maksimal 200 karakter masing masing, hanya karakter Eropa Barat); label yang tidak diberikan tetap Bahasa Inggris.",
                        "properties": {
                            "signed_by": { "type": "string", "description": "Di atas nama. Bawaan \"Digitally signed by\"." },
                            "distinguished_name": { "type": "string", "description": "Sebelum subjek sertifikat. Bawaan \"DN: \"." },
                            "reason": { "type": "string", "description": "Bawaan \"Reason: \"." },
                            "location": { "type": "string", "description": "Bawaan \"Location: \"." },
                            "date": { "type": "string", "description": "Bawaan \"Date: \"." },
                        },
                        "additionalProperties": false,
                    },
                    "out": { "type": "string", "description": "Tempat menyimpan dokumen yang ditandatangani." },
                }),
                &["doc", "id", "out"],
            )),
        t("sign_trust", "Percayai sertifikat", "Tambah sertifikat (.cer/.crt/.pem/.der, bundel .p7b/.p7c, atau sertifikat dalam .p12 dengan `password`) ke sertifikat tepercaya yang dipakai untuk memvalidasi tanda tangan, atau kosongkan daftar (clear: true). Setiap dokumen terbuka divalidasi ulang. Mengembalikan daftar tepercaya.").with(schema(
            json!({ "paths": { "type": "array", "items": { "type": "string" } }, "password": { "type": "string" }, "clear": { "type": "boolean" } }),
            &[],
        )),
        t("comment_mark", "Tandai komentar dengan centang", "Tandai atau hapus tanda komentar dengan centang (Acrobat: Mark with checkmark), dicatat sebagai balasan privat. Dapat dibatalkan.").with(schema(
            comment_ref(json!({ "marked": { "type": "boolean", "description": "Bawaan true." }, "author": { "type": "string" } })),
            &["doc"],
        )),
        t("comment_lock", "Kunci komentar", "Kunci atau buka kunci komentar (Properties ▸ Locked). Komentar terkunci tidak dapat dipindahkan, diubah ukuran, diganti gaya, atau dihapus; teksnya tetap dapat diedit. Dapat dibatalkan.").with(schema(
            comment_ref(json!({ "locked": { "type": "boolean", "description": "Bawaan true." } })),
            &["doc"],
        )),
        t("layer_set", "Tampilkan atau sembunyikan lapisan", "Tampilkan atau sembunyikan lapisan (grup konten opsional), seperti ikon mata panel Layers. layer adalah namanya seperti yang dicantumkan doc_info (setiap lapisan dengan nama itu) atau posisi berbasis 1 dalam daftar itu. Pengaturan tampilan: page_render menggambar dengannya, dan berkas tidak berubah. Mengembalikan setiap lapisan beserta visibilitasnya.")
            .with(schema(
                json!({ "doc": doc(), "layer": { "type": ["string", "integer"], "description": "Nama lapisan, atau posisi berbasis 1 dalam layers doc_info." }, "visible": { "type": "boolean" } }),
                &["doc", "layer", "visible"],
            )),
        t("comments_hide", "Sembunyikan semua komentar", "Sembunyikan atau tampilkan setiap komentar pada halaman (bidang dan tautan tetap digambar). Pengaturan tampilan: berkas tidak berubah.")
            .cmd("comment.hide_all")
            .with(schema(json!({ "doc": doc(), "hidden": { "type": "boolean", "description": "Bawaan true." } }), &["doc"])),
        t("view_reading_colors", "Atur warna baca", "Atur warna baca dokumen (Replace Document Colors Acrobat, dalam Reading Options): cat teks solid dan line-art dipetakan ulang untuk keterbacaan dan halaman menjadi warna latar dalam page_render. Pengaturan tampilan: berkas tidak berubah. mode adalah salah satu dari: off, black-on-white, white-on-black, yellow-on-black, green-on-black, atau custom (yang menerima text dan background sebagai #RRGGBB). only_black hanya memetakan cat yang nyaris hitam (bawaan false) dan line_art juga memetakan cat line-art (bawaan true); keduanya menggantikan preset mode. Tanda yang berbeda dari bawaan preset mode membuat mode hasil terbaca custom, karena parameter hasil tidak lagi cocok dengan preset mana pun. Mengembalikan pengaturan hasil.")
            .cmd("a11y.reading_options")
            .with(schema(
                json!({
                    "doc": doc(),
                    "mode": { "type": "string", "enum": ["off", "black-on-white", "white-on-black", "yellow-on-black", "green-on-black", "custom"] },
                    "text": { "type": "string", "description": "Dengan mode custom: warna teks, #RRGGBB." },
                    "background": { "type": "string", "description": "Dengan mode custom: warna latar, #RRGGBB." },
                    "only_black": { "type": "boolean", "description": "Petakan hanya cat yang nyaris hitam. Bawaan false." },
                    "line_art": { "type": "boolean", "description": "Petakan juga cat line-art. Bawaan true." },
                }),
                &["doc", "mode"],
            )),
        t("comments_summarize", "Ringkas komentar", "Buat PDF yang merangkum setiap komentar (nomor, penulis, jenis, tanggal, teks, balasan), diurutkan menurut halaman, penulis, tanggal, atau jenis.")
            .cmd("comment.summarize")
            .with(schema(
                json!({ "doc": doc(), "sort": { "type": "string", "enum": ["page", "author", "date", "type"] }, "out": save_out, "open": open }),
                &["doc"],
            )),
        t("comment_edit", "Edit komentar", "Ubah teks, warna, opasitas, lebar garis, ukuran font (kotak teks/typewriter/callout), isian (rectangle/oval/polygon/text box/callout; \"none\" menghapusnya), persegi (rectangle/oval/text box/stamp), posisi (`move` [dx, dy] dalam poin), atau bentuk (`points`: dua ujung garis, atau sudut garis tersambung, poligon, atau awan) komentar. Stempel mempertahankan tampilan aslinya saat diubah ukuran. Satu langkah batal.").with(schema(
            comment_ref(json!({
                "contents": { "type": "string" },
                "color": color(),
                "opacity": { "type": "number", "minimum": 0, "maximum": 1 },
                "width": { "type": "number", "minimum": 0 },
                "fill": { "type": "string", "description": "Warna interior persegi, oval, poligon, kotak teks, atau callout: #RRGGBB, nama warna, atau \"none\" untuk tanpa isian." },
                "font_size": { "type": "number", "minimum": 1, "maximum": 400, "description": "Ukuran font dalam poin kotak teks, teks typewriter, atau callout; kotak membesar untuk menampung teks." },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                "move": point(),
                "points": { "type": "array", "items": point(), "minItems": 2, "description": "Dua ujung garis, atau setiap sudut garis tersambung, poligon, atau awan, dalam koordinat halaman." },
            })),
            &["doc"],
        )),
        t("comment_image_preview", "Pratinjau lapisan tanda tangan gambar", "Kembalikan PNG halaman tanpa tanda tangan/inisial gambar terpilih (layer=background, bawaan), atau gambar tertanamnya dengan alpha (layer=image). Mencakup persegi yang ditampilkan, rotasi dokumen, image_rotation (seberapa jauh gambar ditampilkan berputar searah jarum jam), dan opasitas anotasi. Simpan lapisan ini dalam cache untuk penempatan/pengubahan ukuran langsung; terapkan sekali dengan comment_edit. Tidak mengubah dokumen atau riwayat batal.")
            .ro().with(schema(comment_ref(json!({ "layer": { "type": "string", "enum": ["background", "image"], "default": "background" }, "dpi": { "type": "number", "minimum": 1, "maximum": 600, "default": 96 } })), &["doc"])),
        t("comment_delete", "Hapus komentar", "Hapus komentar beserta pop up dan balasannya. Dapat dibatalkan.").destructive().with(schema(comment_ref(json!({})), &["doc"])),
        t(
            "doc_protect",
            "Lindungi dengan kata sandi",
            "Enkripsi dokumen (diterapkan oleh doc_save berikutnya, penulisan ulang penuh). open_password diperlukan untuk membukanya. permissions_password diperlukan untuk mengubah keamanan dan mencabut pembatasan yang diberikan printing/changes/copy/accessibility; pembatasan tersebut (dan bawaannya) hanya berlaku bila permissions_password diberikan. Dengan open_password saja dokumen dienkripsi dan semua tetap diizinkan, sehingga memberikan pembatasan tanpa permissions_password adalah galat. Kata sandi tidak pernah ditampilkan kembali. Dapat dibatalkan.",
        )
        .cmd("protect.password")
        .with(schema(
            json!({
                "doc": doc(),
                "open_password": { "type": "string", "minLength": 1, "description": "Diperlukan untuk membuka dokumen. Sendiri, tidak membatasi apa pun." },
                "permissions_password": { "type": "string", "minLength": 1, "description": "Diperlukan untuk mengubah keamanan; mengaktifkan printing/changes/copy/accessibility dan bawaannya." },
                "printing": { "type": "string", "enum": ["none", "low", "high"], "description": "Membutuhkan permissions_password. Bawaan high." },
                "changes": { "type": "string", "enum": ["none", "pages", "fill-sign", "comment-fill-sign", "any-except-extract"], "description": "Membutuhkan permissions_password. Bawaan none." },
                "copy": { "type": "boolean", "description": "Izinkan menyalin teks dan gambar (membutuhkan permissions_password; bawaan false)." },
                "accessibility": { "type": "boolean", "description": "Izinkan pembaca layar membaca teks (membutuhkan permissions_password; bawaan true)." },
                "compatibility": { "type": "string", "enum": ["aes-256", "aes-128", "rc4-128", "rc4-40"], "description": "Bawaan aes-256 (Acrobat X dan lebih baru)." },
                "encrypt_metadata": { "type": "boolean", "description": "Bawaan true." },
            }),
            &["doc"],
        )),
        t("doc_unprotect", "Hapus keamanan", "Hapus keamanan kata sandi (dokumen harus dibuka dengan kata sandi izinnya, atau tidak memilikinya). Diterapkan oleh doc_save berikutnya. Dapat dibatalkan.")
            .cmd("protect.remove")
            .destructive()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("page_replace", "Ganti halaman", "Ganti konten halaman dengan halaman PDF lain (jumlah sama). Tautan, komentar, bidang formulir, dan markah pada halaman asli tetap, seperti dalam Acrobat. Dapat dibatalkan.")
            .cmd("page.replace")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang diganti"), "path": path_arg(), "from_pages": pages("dari berkas lain, secara berurutan (bawaan: halaman pertamanya)") }), &["doc", "pages", "path"])),
        t("page_duplicate", "Duplikat halaman", "Sisipkan salinan halaman sehingga salinan pertama menjadi halaman `at`, atau sesudah halaman terakhir darinya bila `at` dikosongkan (font dan gambar dipakai bersama, tidak disalin). Dapat dibatalkan.")
            .cmd("page.duplicate")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang diduplikat"), "at": { "type": "integer", "minimum": 1 } }), &["doc", "pages"])),
        t(
            "page_transition_set",
            "Atur transisi halaman",
            "Atur atau hapus transisi dan waktu tampil halaman dalam presentasi layar penuh (ISO 32000-2 §12.4.4; Organize ▸ Page Transitions Acrobat). style: Split, Blinds, Box, Wipe, Dissolve, Glitter, R (replace, tanpa efek), Fly, Push, Cover, Uncover, atau Fade, atau none untuk menghapus transisi; duration dalam detik (bawaan 1); dimension horizontal/vertical (Split, Blinds); motion inward/outward (Split, Box, Fly); direction 0, 90, 180, 270, 315, atau none (Wipe, Glitter, Fly, Cover, Uncover, Push); scale dan opaque untuk Fly. display: detik halaman bertahan sebelum maju; bila dikosongkan, halaman menunggu presenter. doc_info mencantumkan hasil di bawah transitions.",
        )
        .with(schema(
            json!({
                "doc": doc(),
                "pages": pages("yang diubah (bawaan: semua)"),
                "style": { "type": "string", "enum": ["none", "Split", "Blinds", "Box", "Wipe", "Dissolve", "Glitter", "R", "Fly", "Push", "Cover", "Uncover", "Fade"] },
                "duration": { "type": "number", "minimum": 0 },
                "dimension": { "type": "string", "enum": ["horizontal", "vertical"] },
                "motion": { "type": "string", "enum": ["inward", "outward"] },
                "direction": { "description": "0, 90, 180, 270, 315 or \"none\"" },
                "scale": { "type": "number", "exclusiveMinimum": 0 },
                "opaque": { "type": "boolean" },
                "display": { "type": "number", "minimum": 0 },
            }),
            &["doc"],
        )),
        t(
            "page_set_box",
            "Atur kotak halaman / pangkas",
            "Atur kotak halaman (crop sebagai bawaan; juga trim, bleed, art, media) pada pages: baik margin dalam poin dari kotak media [left, bottom, right, top], atau rect absolut dalam poin dari kiri atas halaman yang ditampilkan. Kosongkan keduanya untuk mengatur ulang kotak ke bawaannya. Dapat dibatalkan.",
        )
        .cmd("page.boxes")
        .with(schema(
            json!({
                "doc": doc(),
                "pages": pages("yang diubah (bawaan: semua)"),
                "box": { "type": "string", "enum": ["crop", "trim", "bleed", "art", "media"] },
                "margins": { "type": "array", "items": { "type": "number", "minimum": 0 }, "minItems": 4, "maxItems": 4 },
                "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
            }),
            &["doc"],
        )),
        t(
            "doc_header_footer",
            "Tambah header dan footer",
            "Tambah header dan/atau footer ke halaman. Kotak teks: header_left/center/right, footer_left/center/right. Token: <<1>> nomor halaman, <<n>> jumlah halaman, <<1 of n>>, <<Page 1 of n>>, <<1/n>>, tanggal <<m/d/yyyy>> <<yyyy-mm-dd>> <<mmmm d, yyyy>> (dan lainnya), Bates <<Bates Number#6#1#PREFIX#SUFFIX>>. shrink: true mengecilkan konten halaman secara merata ke ruang yang disisakan header dan footer, agar tidak tertutup. font: Helvetica (bawaan), Times, atau Courier. replace: true mengganti yang ada (Update). Dapat dibatalkan.",
        )
        .cmd("edit.header_footer")
        .with(schema(
            json!({
                "doc": doc(),
                "pages": pages("yang ditandai (bawaan: semua)"),
                "header_left": { "type": "string" }, "header_center": { "type": "string" }, "header_right": { "type": "string" },
                "footer_left": { "type": "string" }, "footer_center": { "type": "string" }, "footer_right": { "type": "string" },
                "font": { "type": "string", "enum": ["Helvetica", "Times", "Courier"] },
                "font_size": { "type": "number", "exclusiveMinimum": 0 },
                "color": { "type": "string", "description": "#RRGGBB atau nama warna." },
                "margins": { "type": "array", "items": { "type": "number", "minimum": 0 }, "minItems": 4, "maxItems": 4, "description": "Atas, bawah, kiri, kanan dalam poin (bawaan 36, 36, 72, 72)." },
                "start_number": { "type": "integer", "minimum": 1 },
                "shrink": { "type": "boolean", "description": "Kecilkan konten halaman agar header dan footer tidak menutupinya (bawaan false)." },
                "replace": { "type": "boolean" },
            }),
            &["doc"],
        )),
        t("doc_watermark", "Tambah watermark", "Tambah watermark ke halaman: teks, atau gambar dari `file` (gambar, atau halaman `file_page` dari PDF) pada `scale` halaman (bawaan 0.5); diputar (derajat berlawanan arah jarum jam, bawaan 45), semi transparan (opacity 0–1, bawaan 0.5), teks dipaskan ke halaman kecuali font_size diberikan, di atas kecuali behind: true; digeser dari titik tengah halaman sebesar `offset_x` (kanan) dan `offset_y` (atas) poin (bawaan 0). replace: true mengganti yang ada. Dapat dibatalkan.")
            .cmd("edit.watermark")
            .with(schema(
                json!({
                    "doc": doc(),
                    "pages": pages("yang ditandai (bawaan: semua)"),
                    "text": { "type": "string", "minLength": 1 },
                    "file": { "type": "string" },
                    "file_page": { "type": "integer", "minimum": 1 },
                    "scale": { "type": "number", "exclusiveMinimum": 0, "maximum": 1 },
                    "font_size": { "type": "number", "exclusiveMinimum": 0 },
                    "color": { "type": "string" },
                    "opacity": { "type": "number", "minimum": 0, "maximum": 1 },
                    "rotation": { "type": "number" },
                    "behind": { "type": "boolean" },
                    "offset_x": { "type": "number", "description": "Poin di kanan titik tengah halaman (negatif: kiri)." },
                    "offset_y": { "type": "number", "description": "Poin di atas titik tengah halaman (negatif: bawah)." },
                    "replace": { "type": "boolean" },
                }),
                &["doc"],
            )),
        t("doc_background", "Tambah latar", "Isi latar halaman dengan warna, atau gambar dari `file` (gambar, atau halaman `file_page` dari PDF) yang dipaskan pada `scale` (bawaan 1), di belakang konten. Dapat dibatalkan.")
            .cmd("edit.background")
            .with(schema(
                json!({ "doc": doc(), "pages": pages("yang diisi (bawaan: semua)"), "color": { "type": "string" }, "file": { "type": "string" }, "file_page": { "type": "integer", "minimum": 1 }, "scale": { "type": "number", "exclusiveMinimum": 0, "maximum": 1 }, "opacity": { "type": "number", "minimum": 0, "maximum": 1 }, "replace": { "type": "boolean" } }),
                &["doc"],
            )),
        t("doc_remove_marks", "Hapus header dan footer, watermark, atau latar", "Hapus setiap header dan footer, watermark, atau latar yang ditambah PdfCraft (atau alat yang kompatibel). Dapat dibatalkan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "kind": { "type": "string", "enum": ["header_footer", "watermark", "background"] } }), &["doc", "kind"])),
        t("doc_export_images", "Ekspor halaman sebagai gambar", "Tulis halaman sebagai berkas PNG, JPEG, atau TIFF (`<name>_page_<n>.png|jpg|tif`) ke folder, pada satu resolusi (bawaan 150 dpi). JPEG dan TIFF diratakan ke kertas putih. Halaman yang terlalu besar untuk renderer pada resolusi itu digambar pada batas maksimumnya, dicantumkan dalam `lower_dpi` ({page, dpi}). Mencakup edit yang belum disimpan.")
            .cmd("export.image")
            .with(schema(
                json!({
                    "doc": doc(),
                    "folder": { "type": "string" },
                    "format": { "type": "string", "enum": ["png", "jpeg", "tiff"] },
                    "quality": { "type": "integer", "minimum": 1, "maximum": 100, "description": "Kualitas JPEG (bawaan 85)." },
                    "dpi": { "type": "number", "minimum": 18, "maximum": 1200 },
                    "pages": pages("yang diekspor (bawaan: semua)"),
                }),
                &["doc", "folder"],
            )),
        t("accessibility_autotag_preview", "Pratinjau tag otomatis", "Usulkan tag paragraf dan heading untuk blok teks yang terlihat pada satu halaman. Mengembalikan batas halaman, peran sementara, dan alasan. Tidak mengubah PDF atau mengklaim kepatuhan aksesibilitas. Gambar, tabel, daftar, anotasi, dan struktur yang ada membutuhkan tinjauan terpisah.")
            .ro()
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 } }), &["doc", "page"])),
        t("accessibility_autotag_apply", "Tag otomatis PDF khusus teks", "Tulis konten bertanda, elemen struktur Document, tag paragraf, dan pohon induk untuk setiap objek teks dalam PDF khusus teks yang belum bertag. Menolak grafis, gambar, anotasi, formulir, tag yang ada, dan stream rusak sebelum mengubah apa pun. Tinjau semua peran hasil; ini tidak menyatakan aksesibilitas. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("accessibility_autotag_form_fields", "Tag bidang formulir", "Tulis tag `/Form`, kunci induk struktur widget, dan pohon induk untuk semua widget halaman dalam AcroForm yang belum bertag. Pohon tag yang ada ditolak untuk ditinjau. Teks, gambar, tautan, dan konten lain tidak ditag.")
            .cmd("a11y.autotag_fields")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("accessibility_check", "Periksa aksesibilitas", "Jalankan pemeriksaan penuh Accessibility Checker (32 aturan dalam 7 kategori: document, page_content, forms, alternate_text, tables, lists, headings). Tiap aturan berstatus lulus, gagal (dengan temuan dan halaman), manual (membutuhkan orang), atau dilewati. Temuan pada gambar, anotasi, dan widget formulir yang dapat ditemukan mencakup bbox_user_space dalam koordinat PDF. Kontras warna nonaktif kecuali all bernilai true; rules (id seperti tagged-pdf, figures-alt-text) atau categories mempersempit pemeriksaan; pages membatasi aturan halaman.")
            .ro()
            .cmd("a11y.check")
            .with(schema(
                json!({
                    "doc": doc(),
                    "rules": { "type": "array", "items": { "type": "string" } },
                    "categories": { "type": "array", "items": { "type": "string", "enum": ["document", "page_content", "forms", "alternate_text", "tables", "lists", "headings"] } },
                    "all": { "type": "boolean", "description": "Sertakan aturan yang nonaktif sebagai bawaan (kontras warna)." },
                    "pages": pages("untuk aturan halaman (bawaan: semua)"),
                }),
                &["doc"],
            )),
        t("accessibility_find_untagged", "Temukan konten tanpa tag", "Temukan konten halaman tergambar, anotasi, dan widget formulir tanpa tag. Pemeriksaan terfokus ini tidak mengubah PDF atau mengklaim aturan aksesibilitas lainnya lulus.")
            .ro()
            .cmd("a11y.find_untagged")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang dicari (bawaan: semua)") }), &["doc"])),
        t("accessibility_report", "Laporan aksesibilitas", "Jalankan pemeriksaan penuh dan tulis laporan aksesibilitas (HTML) ke path; mengembalikan hasil juga. Memakai opsi yang sama dengan accessibility_check.")
            .destructive()
            .cmd("a11y.report")
            .with(schema(
                json!({
                    "doc": doc(),
                    "path": { "type": "string" },
                    "rules": { "type": "array", "items": { "type": "string" } },
                    "categories": { "type": "array", "items": { "type": "string" } },
                    "all": { "type": "boolean" },
                    "pages": pages("untuk aturan halaman (bawaan: semua)"),
                }),
                &["doc", "path"],
            )),
        t("accessibility_review_record", "Catatan tinjauan aksesibilitas", "Bandingkan aturan aksesibilitas yang sama pada dua versi terbuka PDF dan tulis catatan JSON ke path. Mencakup hash SHA-256 dokumen, status aturan sebelum/sesudah dan jumlah temuan, aturan yang berubah, dan aturan yang masih membutuhkan tinjauan. Tidak mengubah PDF mana pun atau mengklaim kepatuhan aksesibilitas. Buka salinan asli dan salinan edit sebagai dokumen terpisah.")
            .destructive()
            .with(schema(
                json!({
                    "before": doc(),
                    "after": doc(),
                    "path": { "type": "string" },
                    "all": { "type": "boolean", "description": "Sertakan aturan yang nonaktif sebagai bawaan (kontras warna)." },
                }),
                &["before", "after", "path"],
            )),
        t("accessibility_review_save", "Simpan keputusan tinjauan aksesibilitas", "Tulis berkas pendamping JSON lokal untuk PDF terbuka dengan keputusan tinjauan manusia tingkat aturan. Berkas pendamping mencatat SHA-256 PDF; penggantian berkas yang terkait byte berbeda membutuhkan replace: true. Tidak mengubah PDF atau menyatakan kepatuhan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" }, "decisions": { "type": "array", "items": { "type": "object", "properties": { "rule": { "type": "string" }, "decision": { "type": "string", "enum": ["accepted", "needs_work", "skipped"] }, "page": { "type": "integer", "minimum": 1 }, "note": { "type": "string" } }, "required": ["rule", "decision"], "additionalProperties": false } }, "replace": { "type": "boolean" } }), &["doc", "path", "decisions"])),
        t("accessibility_review_load", "Buka keputusan tinjauan aksesibilitas", "Baca berkas pendamping tinjauan lokal hanya bila SHA-256 yang tercatat cocok dengan PDF terbuka. Mengembalikan keputusan tanpa mengubah PDF.")
            .ro()
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" } }), &["doc", "path"])),
        t("accessibility_review_html", "Ringkasan tinjauan aksesibilitas", "Bandingkan dua versi terbuka PDF dan tulis ringkasan HTML mandiri dari bukti yang sama dengan accessibility_review_record: hash dokumen, jumlah hasil, perubahan aturan, dan aturan yang masih membutuhkan tinjauan. Tidak mengubah PDF mana pun atau mengklaim kepatuhan aksesibilitas.")
            .destructive()
            .with(schema(
                json!({
                    "before": doc(),
                    "after": doc(),
                    "path": { "type": "string" },
                    "all": { "type": "boolean", "description": "Sertakan aturan yang nonaktif sebagai bawaan (kontras warna)." },
                }),
                &["before", "after", "path"],
            )),
        t("accessibility_fix", "Perbaiki masalah aksesibilitas", "Terapkan perbaikan otomatis pemeriksa untuk satu aturan: primary-language (value: bahasa, mis. en-US), title (value: judul; bawaan judul saat ini atau nama berkas; juga menampilkannya pada bilah judul), atau tab-order (tiap halaman bertab dalam urutan struktur). Mengembalikan status baru aturan. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "rule": { "type": "string", "enum": ["primary-language", "title", "tab-order"] }, "value": { "type": "string" } }), &["doc", "rule"])),
        t("accessibility_figures", "Daftar gambar", "Gambar bertag (elemen Figure, melalui peta peran) dalam urutan dokumen: nomor gambar, halaman, teks alternatif, dan tempat gambar digambar (poin bertitik asal kiri atas).")
            .ro()
            .cmd("a11y.alt_text")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("accessibility_set_alt", "Atur teks alternatif", "Atur teks alternatif gambar (alt; kosong atau dikosongkan menghapusnya), atau tandai dekoratif (decorative: true: kontennya menjadi artifact dan gambar keluar dari tag). figure adalah nomor dari accessibility_figures. Mengembalikan gambar. Dapat dibatalkan.")
            .cmd("a11y.alt_text")
            .with(schema(json!({ "doc": doc(), "figure": { "type": "integer", "minimum": 1 }, "alt": { "type": "string" }, "decorative": { "type": "boolean" } }), &["doc", "figure"])),
        t("accessibility_tags", "Daftar tag struktur", "Daftar tag struktur tidak langsung yang dapat dijangkau dalam urutan dokumen dengan nomor objek PDF, generation, induk tidak langsung terdekat, kedalaman, halaman, jenis, judul, teks aktual, teks alternatif, bahasa, ringkasan tabel, dan scope sel header, satu halaman dalam satu waktu: count adalah berapa tag yang cocok, next offset halaman berikutnya (null pada akhir). types, pages, dan max_depth hanya mempertahankan sebagian tag. Dictionary tag sebaris tidak memiliki referensi objek yang stabil dan tidak dicantumkan.")
            .ro()
            .cmd("a11y.tags")
            .with(schema(
                json!({
                    "doc": doc(),
                    "offset": { "type": "integer", "minimum": 0, "description": "Posisi awal halaman, dihitung dari 0 (next dari halaman sebelumnya)" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Tag dalam halaman (bawaan 200)" },
                    "types": { "type": "array", "items": { "type": "string" }, "description": "Hanya tag berjenis ini, seperti yang dicantumkan dalam type (seperti H1, H2, Table)" },
                    "pages": { "type": "array", "items": { "type": "integer", "minimum": 1 }, "description": "Hanya tag pada halaman ini" },
                    "max_depth": { "type": "integer", "minimum": 0, "description": "Hanya tag dengan kedalaman maksimal ini (0 = tingkat teratas)" }
                }),
                &["doc"],
            )),
        t("accessibility_tag_content", "Daftar konten tag", "Daftar referensi konten bertanda yang dimiliki langsung oleh satu tag struktur tidak langsung yang dapat dijangkau, mencakup halaman, MCID, dan teks hasil ekstrak.")
            .ro()
            .cmd("a11y.tags")
            .with(schema(json!({ "doc": doc(), "object": { "type": "integer", "minimum": 1 }, "generation": { "type": "integer", "minimum": 0, "maximum": 65535 } }), &["doc", "object", "generation"])),
        t("accessibility_set_tag_property", "Edit properti tag", "Atur atau hapus satu properti tag struktur tidak langsung yang dapat dijangkau. Gunakan object dan generation dari accessibility_tags. Properti: type, title, actual_text, alternate_text, language, summary (tag Table: atribut Summary tabel), scope (tag TH: Row, Column, atau Both). Summary dan scope ditulis dalam objek atribut Table tag (/A, /O /Table); peta peran menentukan jenis tag. Nilai kosong atau dikosongkan menghapus properti teks, summary, dan scope; type membutuhkan nama PDF yang tidak kosong. Dapat dibatalkan.")
            .with(schema(json!({
                "doc": doc(), "object": { "type": "integer", "minimum": 1 },
                "generation": { "type": "integer", "minimum": 0, "maximum": 65535 },
                "property": { "type": "string", "enum": ["type", "title", "actual_text", "alternate_text", "language", "summary", "scope"] },
                "value": { "type": "string" },
            }), &["doc", "object", "generation", "property"])),
        t("accessibility_mark_tag_artifact", "Tandai konten tag sebagai artifact", "Tandai konten bertanda langsung milik tag struktur daun yang dapat dijangkau sebagai artifact dan hapus tag dari struktur baca. Gunakan object dan generation dari accessibility_tags. Tag dengan elemen struktur anak, tanpa konten bertanda langsung, atau konten yang tidak ditemukan pada halamannya ditolak. Dapat dibatalkan.")
            .with(schema(json!({
                "doc": doc(), "object": { "type": "integer", "minimum": 1 },
                "generation": { "type": "integer", "minimum": 0, "maximum": 65535 },
            }), &["doc", "object", "generation"])),
        t("accessibility_move_tag", "Pindahkan tag struktur", "Pindahkan tag struktur tidak langsung yang dapat dijangkau sebelum atau sesudah saudara langsung dalam array /K induknya, mengubah urutan baca bertag tanpa mengubah tampilan halaman. Gunakan nilai object dan generation dari accessibility_tags. Menolak induk yang berbeda, referensi ganda, dan posisi yang sudah saat ini. Dapat dibatalkan.")
            .with(schema(json!({
                "doc": doc(), "object": { "type": "integer", "minimum": 1 },
                "generation": { "type": "integer", "minimum": 0, "maximum": 65535 },
                "anchor": { "type": "integer", "minimum": 1 },
                "anchor_generation": { "type": "integer", "minimum": 0, "maximum": 65535 },
                "placement": { "type": "string", "enum": ["before", "after"] },
            }), &["doc", "object", "generation", "anchor", "anchor_generation", "placement"])),
        t("form_merge_data", "Gabungkan berkas data menjadi spreadsheet", "Kumpulkan nilai bidang berkas data formulir (FDF, XFDF) atau formulir PDF yang terisi menjadi satu berkas CSV pada path: satu kolom per nama bidang, satu baris per berkas. Mengembalikan jumlah baris dan kolom.")
            .cmd("form.merge_data")
            .with(schema(json!({ "paths": { "type": "array", "items": { "type": "string" }, "minItems": 1 }, "path": { "type": "string" } }), &["paths", "path"])),
        t("js_run", "Jalankan JavaScript", "Jalankan JavaScript Acrobat dalam dokumen, seperti yang dilakukan konsol JavaScript (atau sebagai skrip Mouse Up tombol tekan `field` bila field diberikan; pada formulir XFA yang tertata, `field` menjalankan skrip klik XFA tombol itu sebagai gantinya, JavaScript atau FormCalc, yang dapat menambah dan menghapus baris serta menampilkan atau menyembunyikan subform). Model objek formulir tersedia: this/getField, event, app, util, console, display, color, dan skrip tingkat dokumen (kecuali skrip penampung PDF saat tata letak XFA aktif; tetap tersedia melalui js_document_scripts). Perubahan bidang dan resetForm diterapkan sebagai satu langkah yang dapat dibatalkan; mengembalikan alerts skrip, keluaran konsol, requests (print, page, url, submit), error, dan galat skrip tingkat dokumen yang gagal (sisanya tetap berjalan).")
            .cmd("tools.js_console")
            .with(schema(
                json!({ "doc": doc(), "script": { "type": "string" }, "field": { "type": "string", "description": "Jalankan sebagai peristiwa Mouse Up tombol ini." } }),
                &["doc", "script"],
            )),
        t("js_document_scripts", "JavaScript dokumen", "Daftar JavaScript tingkat dokumen (nama dan sumber), yang menentukan fungsi yang dipakai skrip bidang.")
            .ro()
            .cmd("tools.document_js")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("js_set_document_script", "Edit JavaScript dokumen", "Tambah atau ganti JavaScript tingkat dokumen `name` dengan `script`, atau hapus (script dikosongkan). Dapat dibatalkan.")
            .cmd("tools.document_js")
            .with(schema(json!({ "doc": doc(), "name": { "type": "string", "minLength": 1 }, "script": { "type": "string" } }), &["doc", "name"])),
        t("form_set_script", "Atur JavaScript bidang", "Field Properties ▸ Run custom script: atur JavaScript field untuk peristiwa keystroke, format, validate, atau calculate (tindakan bidang; bidang terhitung bergabung ke urutan perhitungan) atau mouse_up (tindakan tombol). Kosongkan script untuk menghapusnya. Skrip memakai model objek Acrobat (event.value, event.rc, getField, util.printf, …) dan berjalan saat bidang berubah. Dapat dibatalkan.")
            .cmd("form.prepare")
            .with(schema(
                json!({ "doc": doc(), "field": { "type": "string" }, "event": { "type": "string", "enum": ["keystroke", "format", "validate", "calculate", "mouse_up"] }, "script": { "type": "string" } }),
                &["doc", "field", "event"],
            )),
        t("doc_export_office", "Ekspor ke Word, Excel, PowerPoint, HTML, atau RTF", "Ekspor PDF ▸ Word (.docx), Excel (.xlsx), PowerPoint (.pptx), HTML (.html, satu berkas dengan gambar sebaris), atau RTF (.rtf), dipilih menurut ekstensi path: teks, heading, tebal, dan miring. Word memakai tabel inferensi tanpa batas dan bentuk yang dapat diedit yang diposisikan pada halaman dengan karya, teks terputar, atau teks besar; tiap halaman sumber mempertahankan ukurannya. Excel menulis satu lembar kerja per halaman: tabel terdeteksi sebagai kisi sel (sel membentang digabung, angka polos sebagai angka), paragraf lain satu per baris dalam kolom A; gambar ditinggalkan. PowerPoint menulis satu slide per halaman (berukuran seperti halaman pertama) dengan tiap paragraf sebagai kotak teks, tiap gambar sebagai gambar, dan tiap tabel terdeteksi sebagai tabel, pada posisi halamannya. Pencocokan font dan grafis vektor tetap terbatas. Ekspor membaca form sedalam 50 dan hingga 50 000 per halaman, menghitung form yang digambar halaman itu sendiri. Halaman yang melewati salah satu batas dicantumkan dalam truncated_pages, dengan peringatan; teks dan gambar form yang ditinggalkan hilang.")
            .cmd("export.docx")
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" } }), &["doc", "path"])),
        t("pdfa_verify", "Verifikasi PDF/A", "Standards ▸ Verify PDF/A compliance: aturan PDF/A-2b atau 3b yang dilanggar dokumen (klausul ISO 19005, pesan, halaman, apakah Save as PDF/A dapat memperbaikinya), serta yang dinyatakannya.")
            .ro()
            .cmd("standards.pdfa")
            .with(schema(json!({ "doc": doc(), "level": { "type": "string", "enum": ["2b", "3b"], "description": "Bawaan 2b." } }), &["doc"])),
        t("print_readiness", "Kesiapan cetak", "Hanya verifikasi. Melaporkan resolusi gambar efektif tempat tiap gambar digambar (bukan terhadap ukuran halaman), kotak halaman dan bleed melewati trim, output intent, font yang tidak tertanam, dan hasil struktural PDF/A-2b. Tidak mengubah dokumen, menanam font, mengarang profil cetak, atau memeriksa PDF/X.")
            .ro()
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("pdfa_convert", "Simpan sebagai PDF/A", "Standards ▸ Save as PDF/A: perbaiki yang dapat diperbaiki untuk PDF/A-2b atau 3b (identifikasi dan metadata XMP, output intent sRGB, tindakan dan JavaScript terlarang, tanda cetak anotasi, interpolasi gambar, enkripsi). Mengembalikan yang diperbaiki dan yang tersisa (mis. font yang tidak tertanam). Dapat dibatalkan; simpan dokumen untuk mempertahankannya.")
            .cmd("standards.pdfa")
            .with(schema(json!({ "doc": doc(), "level": { "type": "string", "enum": ["2b", "3b"] } }), &["doc"])),
        t("standards_remove_id", "Hapus identifikasi PDF/A atau PDF/X", "Hapus yang mengidentifikasi dokumen sebagai PDF/A (properti XMP pdfaid:) atau PDF/X (GTS_PDFXVersion dan GTS_PDFXConformance, dalam informasi dokumen dan sebagai properti XMP pdfxid:), mis. sesudah edit dokumen tidak lagi memenuhinya. Output intent dan metadata lain tetap. Gagal bila dokumen tidak menyatakan hal semacam itu. Dapat dibatalkan.")
            .with(schema(json!({ "doc": doc(), "standard": { "type": "string", "enum": ["pdfa", "pdfx"] } }), &["doc", "standard"])),
        t("action_list", "Daftar tindakan", "Action Wizard: tindakan bawaan (name, description, steps) dan setiap langkah yang dapat dipakai tindakan (id, label, apakah menerima argumen teks).")
            .ro()
            .cmd("actions.wizard")
            .with(schema(json!({}), &[])),
        t("action_run", "Jalankan tindakan", "Action Wizard: jalankan tindakan bawaan (action: namanya) atau daftar langkah ([{step, arg}], id dari action_list) pada tiap PDF dalam paths, menulis hasil ke folder dengan nama yang sama. Mengembalikan catatan langkah atau galat tiap berkas.")
            .cmd("actions.wizard")
            .with(schema(
                json!({
                    "action": { "type": "string" },
                    "steps": { "type": "array", "items": { "type": "object", "properties": { "step": { "type": "string" }, "arg": { "type": "string" } }, "required": ["step"] } },
                    "paths": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
                    "folder": { "type": "string" },
                }),
                &["paths", "folder"],
            )),
        t("doc_compare", "Bandingkan berkas", "Bandingkan teks dua dokumen terbuka: other adalah versi lama, doc versi baru. Mengembalikan jumlah dan tiap perubahan (diganti, disisipkan, dihapus) dengan teks lama dan baru, halaman (berbasis 1), dan persegi (poin, titik asal kiri bawah).")
            .ro()
            .cmd("doc.compare")
            .with(schema(json!({ "doc": doc(), "other": { "type": "integer", "description": "Dokumen lama (dari doc_open)." }, "limit": { "type": "integer", "minimum": 1, "description": "Cantumkan maksimal perubahan sebanyak ini (bawaan 500)." }, "visual": { "type": "boolean", "description": "Bandingkan juga tampilan halaman (halaman n dengan halaman n): wilayah yang berbeda secara visual." } }), &["doc", "other"])),
        t("doc_compare_report", "Laporan perbandingan", "Tulis laporan ringkasan perbandingan (PDF yang mencantumkan setiap perubahan) ke path.")
            .cmd("doc.compare")
            .with(schema(json!({ "doc": doc(), "other": { "type": "integer" }, "path": { "type": "string" } }), &["doc", "other", "path"])),
        t("doc_compare_mark", "Tandai perbedaan sebagai komentar", "Tambah perbedaan dari other (lama) ke doc (baru) sebagai komentar dalam doc: sorotan di atas teks yang diganti (biru) dan disisipkan (hijau), catatan tempat teks dihapus (merah), dibuat oleh Compare. Dapat dibatalkan.")
            .cmd("doc.compare")
            .with(schema(json!({ "doc": doc(), "other": { "type": "integer" } }), &["doc", "other"])),
        t("form_detect_fields", "Deteksi bidang formulir", "Prepare a form ▸ automatic field detection: temukan bagian kosong yang diminta formulir cetak untuk diisi (deretan garis bawah, garis, kotak kosong, baris kotak sisir, persegi kecil untuk kotak centang; pada halaman pindaian dibaca dari piksel) dan namai tiap bidang dari labelnya. Dengan add (bawaan true) bidang dibuat sebagai satu langkah yang dapat dibatalkan; jika tidak, hanya diusulkan. Mengembalikan page (berbasis 1), kind, name, dan rect (poin, titik asal kiri bawah); satu baris kotak sisir adalah satu bidang teks dengan comb diatur ke jumlah selnya, dibuat dengan tanda comb dan batas karakter tersebut.")
            .cmd("form.detect")
            .with(schema(json!({ "doc": doc(), "pages": pages("yang diperiksa (bawaan: semua)"), "add": { "type": "boolean" } }), &["doc"])),
        t("form_actions", "Tindakan bidang", "Field Properties ▸ Actions: tindakan bidang untuk tiap pemicu (mouse_up, mouse_down, mouse_enter, mouse_exit, on_focus, on_blur).")
            .ro()
            .cmd("form.prepare")
            .with(schema(json!({ "doc": doc(), "field": { "type": "string" } }), &["doc", "field"])),
        t("form_set_actions", "Atur tindakan bidang", "Field Properties ▸ Actions: ganti tindakan bidang. Tiap item: trigger (mouse_up, mouse_down, mouse_enter, mouse_exit, on_focus, on_blur) dan salah satu dari javascript (sumber), url, reset (nama bidang; [] untuk semua), menu (Print, NextPage, PrevPage, FirstPage, LastPage), page (berbasis 1), show / hide (nama bidang), submit (URL). Pemicu yang tidak dicantumkan kehilangan tindakannya. Dapat dibatalkan.")
            .cmd("form.prepare")
            .with(schema(
                json!({ "doc": doc(), "field": { "type": "string" }, "actions": { "type": "array", "items": { "type": "object", "properties": {
                    "trigger": { "type": "string", "enum": ["mouse_up", "mouse_down", "mouse_enter", "mouse_exit", "on_focus", "on_blur"] },
                    "javascript": { "type": "string" }, "url": { "type": "string" }, "reset": { "type": "array", "items": { "type": "string" } },
                    "menu": { "type": "string" }, "page": { "type": "integer", "minimum": 1 }, "show": { "type": "array", "items": { "type": "string" } },
                    "hide": { "type": "array", "items": { "type": "string" } }, "submit": { "type": "string" } }, "required": ["trigger"] } } }),
                &["doc", "field", "actions"],
            )),
        t("js_enabled", "JavaScript aktif atau nonaktif", "Preferences ▸ JavaScript ▸ Enable Acrobat JavaScript: atur dengan `enabled`, atau baca. Dengan JavaScript nonaktif, skrip bidang selain panggilan AF Acrobat tidak berjalan.")
            .with(schema(json!({ "enabled": { "type": "boolean" } }), &[])),
        t("ocr_recognize", "Kenali teks (OCR)", "Scan & OCR ▸ Recognize text: render halaman, baca kata di dalamnya dan tambahkan sebagai teks tidak terlihat di atas gambar halaman, sehingga halaman pindaian menjadi dapat dicari dan dipilih (gambar yang dapat dicari; gambar tidak diubah). Halaman yang sudah memiliki teks dilewati kecuali skip_text_pages bernilai false; maka gambar pada halaman itu juga dibaca, dan kata yang dikenali yang sudah dimiliki teks halaman itu sendiri (termasuk lapisan OCR sebelumnya) ditinggalkan dan dihitung sebagai already_on_page. Mengembalikan teks yang dikenali tiap halaman (ditinggalkan bila keamanan dokumen tidak mengizinkan penyalinan), jumlah kata, atau alasan dilewati. Membutuhkan model OCR (ocr_status). Dapat dibatalkan sebagai satu langkah.")
            .cmd("ocr.recognize")
            .with(schema(
                json!({
                    "doc": doc(),
                    "pages": pages("yang dikenali (bawaan: semua)"),
                    "dpi": { "type": "number", "minimum": 72, "maximum": 600, "description": "Resolusi pembacaan halaman (bawaan 300; halaman pindaian dibaca maksimal pada resolusi aslinya)." },
                    "language": { "type": "string", "enum": ["en"], "description": "Bahasa dokumen (bawaan en)." },
                    "skip_text_pages": { "type": "boolean", "description": "Biarkan halaman yang sudah berisi teks (bawaan true). false juga membaca gambar pada halaman semacam itu, tanpa menambah teksnya lagi." },
                }),
                &["doc"],
            )),
        t("ocr_recognize_files", "Kenali teks dalam banyak berkas", "Scan & OCR ▸ Recognize text ▸ In multiple files: baca tiap halaman tiap PDF dalam paths dan tulis hasil yang dapat dicari ke folder dengan nama yang sama (halaman yang sudah memiliki teks dibiarkan kecuali skip_text_pages bernilai false, seperti dalam ocr_recognize). Dua masukan yang akan menulis berkas keluaran yang sama ditolak sebelum apa pun dibaca atau ditulis. Mengembalikan, per berkas, jalur keluaran, jumlah kata, dan halaman yang dilewati, atau galat.")
            .cmd("ocr.recognize_batch")
            .with(schema(
                json!({
                    "paths": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
                    "folder": { "type": "string" },
                    "dpi": { "type": "number", "minimum": 72, "maximum": 600 },
                    "language": { "type": "string", "enum": ["en"] },
                    "skip_text_pages": { "type": "boolean", "description": "Biarkan halaman yang sudah berisi teks (bawaan true); lihat ocr_recognize." },
                }),
                &["paths", "folder"],
            )),
        t("ocr_status", "Status OCR", "Apakah pengenalan teks tersedia (modelnya terpasang: paket rilis menyertakannya; build sumber mengambilnya dengan `cargo xtask models`; PDFCRAFT_MODELS menggantikannya), tempat mencarinya, dan bahasa yang dibacanya.")
            .ro()
            .cmd("ocr.recognize")
            .with(schema(json!({}), &[])),
        t("doc_export_all_images", "Ekspor semua gambar", "Tulis gambar yang dipakai halaman ke folder (`<name>_Page_<n>_Image_<k>.jpg|png`, n diisi nol hingga digit jumlah halaman), masing masing sekali: gambar JPEG tidak berubah, lainnya sebagai PNG dengan mask lembutnya sebagai alpha. min_size melewati gambar dengan piksel lebih sedikit pada sisi terpendeknya. Gambar yang belum dapat didekode (JPEG 2000, JBIG2, CCITT, separations) dicantumkan di bawah skipped. Mencakup edit yang belum disimpan.")
            .cmd("export.all_images")
            .with(schema(
                json!({
                    "doc": doc(),
                    "folder": { "type": "string" },
                    "pages": pages("yang gambarnya diekspor (bawaan: semua)"),
                    "min_size": { "type": "integer", "minimum": 0, "description": "Lewati gambar yang lebih kecil dari piksel sebanyak ini pada sisi terpendeknya (bawaan 0)." },
                }),
                &["doc", "folder"],
            )),
        t("doc_export_text", "Ekspor teks", "Tulis teks urutan baca halaman ke berkas .txt (halaman dipisahkan form feed).")
            .cmd("export.text")
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" }, "pages": pages("yang diekspor (bawaan: semua)") }), &["doc", "path"])),
        t(
            "fill_sign_add",
            "Fill & Sign: ketik teks atau letakkan tanda",
            "Isi formulir yang tidak memiliki bidang, seperti yang dilakukan Fill & Sign Acrobat: ketik teks (`text`, bawaan 10 pt; `font_size` 4–300 pt), letakkan centang, silang, titik, atau garis, tanggal hari ini (dalam Preferences ▸ Date format, lihat fill_sign_date_format, atau `format`; `font_size` juga berlaku), atau tanda tangan atau inisial (baik `text` yang digambar dalam font script, atau `path` ke gambar PNG/JPEG lokal hingga 4 MiB dan 32 megapiksel, dikecilkan ke 4 megapiksel, mempertahankan transparansi; dengan `scan: true`, `path` sebagai gantinya foto tanda tangan di kertas, dalam batas yang sama: tanda tangan ditemukan, dan kertas, bayangan, serta meja di sekitarnya dihapus, dengan `sensitivity` 0–100, bawaan 50, menangkap tinta lebih samar bila lebih tinggi). `at` [x, y] dalam poin dari kiri atas halaman: kiri atas teks, titik tengah tanda, tepi kiri tanda tangan terpusat vertikal. Tanda tangan gambar pas dalam lebar 150 pt dan tinggi 32 pt (24 pt untuk inisial). Membuat anotasi yang dapat dipindahkan dan dibatalkan. Teks ketikan yang tidak dapat digambar font standar disimpan dan digambar sebagai `?` (`warnings` dalam hasil menyatakannya); tanda tangan ketikan yang tidak dapat digambar font script maupun cadangannya ditolak (isi `path` sebagai gantinya).",
        )
        .cmd("sign.fill.text")
        .with(schema(
            json!({
                "doc": doc(),
                "page": { "type": "integer", "minimum": 1 },
                "type": { "type": "string", "enum": ["text", "check", "cross", "dot", "line", "date", "signature", "initials"] },
                "at": point(),
                "text": { "type": "string", "minLength": 1 },
                "font_size": { "type": "number", "minimum": 4, "maximum": 300, "description": "Untuk type text atau date (bawaan 10 pt)." },
                "path": { "type": "string", "description": "PNG atau JPEG untuk signature/initials; isi path atau text." },
                "scan": { "type": "boolean", "description": "Dengan path: pindai foto tanda tangan di kertas (bawaan false)." },
                "sensitivity": { "type": "integer", "minimum": 0, "maximum": 100, "description": "Dengan scan: seberapa samar tinta boleh, 0–100 (bawaan 50)." },
                "format": { "type": "string", "description": "Untuk type date: pola ini sebagai pengganti preferensi format tanggal, mis. \"dd.mm.yyyy\"." },
                "language": { "type": "string", "description": "Untuk type date: nama bulan dan hari dalam kode bahasa ini (lihat fill_sign_date_format) sebagai pengganti preferensi." },
                "author": { "type": "string" },
            }),
            &["doc", "page", "type", "at"],
        )),
        t("fill_sign_date_format", "Format tanggal untuk Fill & Sign", "Preferences ▸ Date format: pola yang dipakai tanggal Fill & Sign (bawaan m/d/yyyy). Atur dengan `format` (kode tanggal Acrobat yyyy yy mmmm mmm mm m dddd ddd dd d, deretan y, m, atau d lain ditolak; H h M s t adalah huruf waktu dan membutuhkan \\ sebelumnya; \\ menampilkan karakter berikutnya apa adanya), atau baca. `language` mengatur bahasa nama bulan dan hari (kode dari `languages`, atau `auto` untuk mengikuti bahasa antarmuka aplikasi; Bahasa Inggris headless). Mengembalikan format, bahasa, hari ini dalam keduanya, `unwritable` (karakter tanggal hari ini yang belum dapat ditulis Fill & Sign ke PDF, hanya Eropa Barat; fill_sign_add menolak tanggal semacam itu), preset siap pakai, dan bahasa.")
            .with(schema(json!({ "format": { "type": "string", "minLength": 1, "maxLength": 64 }, "language": { "type": "string" } }), &[])),
        t(
            "doc_create",
            "Buat PDF",
            "Buat dokumen baru yang belum disimpan dan kembalikan seperti doc_open: `blank` (pages, width, height dalam poin; bawaan 1 halaman US Letter), `images` (paths berkas PNG, JPEG, TIFF (tiap halaman), GIF, atau BMP, satu halaman masing masing pada resolusi gambar) atau `text` (path .txt, atau `text` langsung; hanya karakter Eropa Barat (WinAnsi), lainnya ditolak), atau `office` (`path` berkas Word .docx, Excel .xlsx, PowerPoint .pptx, atau .rtf, dikonversi pada perangkat: halaman, bagian, gaya, daftar, tabel, gambar, header, dan footer Word; tiap lembar Excel yang terlihat sebagai kisi, diskalakan atau diputar lanskap bila lebar; satu halaman per slide dengan tata letak, master, bentuk, gambar, dan tabel; bagan dan SmartArt ditinggalkan, teks diatur dalam font standar dan karakter di luar skrip Eropa Barat tampil sebagai `?`; .doc/.xls/.ppt lama ditolak dengan saran menyimpan sebagai .docx/.xlsx/.pptx). Simpan dengan doc_save dan path.",
        )
        .with(schema(
            json!({
                "from": { "type": "string", "enum": ["blank", "images", "text", "office"] },
                "paths": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
                "dpi": { "type": "number", "minimum": 1, "maximum": 1200, "description": "Untuk gambar: ganti resolusi tertanam tanpa resampling. 72 memberikan satu poin per piksel; kosongkan untuk memakai resolusi tiap gambar (72 bila tidak ada)." },
                "text": { "type": "string" },
                "path": { "type": "string" },
                "pages": { "type": "integer", "minimum": 1, "maximum": 10000 },
                "width": { "type": "number", "minimum": 3 },
                "height": { "type": "number", "minimum": 3 },
                "name": { "type": "string", "description": "Nama untuk dokumen baru (bawaan diturunkan dari sumber)." },
            }),
            &["from"],
        )),
        t("doc_reduce", "Kecilkan ukuran berkas", "Tulis salinan dokumen yang lebih kecil ke `path` dengan pilihan Reduce File Size Acrobat: gambar di atas 225 ppi di-downsample ke 150 ppi dan dikompresi JPEG (kualitas sedang), thumbnail dibuang, font dan gambar identik digabung, objek tidak terpakai dibuang, stream objek terkompresi. Bila salinan tidak lebih kecil, tidak ada yang ditulis dan `written` bernilai false. Dokumen terbuka tidak berubah.")
            .cmd("optimize.reduce")
            .with(schema(json!({ "doc": doc(), "path": { "type": "string" } }), &["doc", "path"])),
        t("doc_initial_view", "Tampilan awal", "Baca atau ubah cara dokumen dibuka (Document Properties ▸ Initial View) dan opsi bacanya: navigation (page, bookmarks, pages, attachments, layers), layout (default, single, continuous, two_up, two_up_continuous, two_up_cover, two_up_continuous_cover), magnification (default, actual, fit_page, fit_width, fit_height, fit_visible, atau persentase), page, opsi jendela (fit_window, center_window, full_screen, display_title), opsi antarmuka (hide_menubar, hide_toolbar, hide_window_ui), language dan binding (left, right). Hanya yang diberikan yang berubah; mengembalikan hasil. Dapat dibatalkan.").with(schema(
            json!({
                "doc": doc(),
                "navigation": { "type": "string", "enum": ["page", "bookmarks", "pages", "attachments", "layers"] },
                "layout": { "type": "string", "enum": ["default", "single", "continuous", "two_up", "two_up_continuous", "two_up_cover", "two_up_continuous_cover"] },
                "magnification": {},
                "page": { "type": "integer", "minimum": 1 },
                "fit_window": { "type": "boolean" },
                "center_window": { "type": "boolean" },
                "full_screen": { "type": "boolean" },
                "display_title": { "type": "boolean" },
                "hide_menubar": { "type": "boolean" },
                "hide_toolbar": { "type": "boolean" },
                "hide_window_ui": { "type": "boolean" },
                "language": { "type": "string" },
                "binding": { "type": "string", "enum": ["left", "right"] },
            }),
            &["doc"],
        )),
        t("doc_audit_space", "Audit pemakaian ruang", "Berapa byte yang dipakai tiap jenis konten dan bagiannya dari berkas (PDF Optimizer ▸ Audit space usage): gambar, stream konten, font, formulir, komentar, struktur, markah, … dan overhead dokumen.")
            .ro()
            .cmd("optimize.advanced")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("text_lines", "Daftar baris teks", "Baris teks yang ada pada halaman (Edit a PDF ▸ Edit text): nomor, teks, kotak (poin bertitik asal kiri atas), font, dan ukuran. Gunakan nomor dengan text_edit.")
            .ro()
            .cmd("edit.edit_text")
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 } }), &["doc", "page"])),
        t("image_hit", "Uji kena karya halaman", "Daftar nomor image/Form yang penempatan dan semua jalur pemotongannya memuat titik [x,y] dalam poin bertitik asal kiri atas. Kurva Bezier, isian even-odd/nonzero, dan lubang diperhatikan. Baca saja.")
            .ro()
            .cmd("edit.edit_text")
            .with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1},"point":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2}}), &["doc","page","point"])),
        t("page_images", "Daftar gambar halaman", "Gambar raster dan karya Form berkelompok yang digambar halaman (Edit a PDF): nomor, rect penempatan penuh dan visible_rect sesudah pemotongan halaman/jalur (poin bertitik asal kiri atas; null bila tersembunyi penuh), kind (image/form), ukuran piksel ([0,0] untuk form), dan nama resource. Form mencakup grafis bersarang dan diedit sebagai satu kesatuan. Nomor menghitung form dan gambar bersama dalam urutan gambar, sehingga halaman dengan form menomori gambarnya berbeda dari sebelum form dicantumkan: selalu ambil nomor dari panggilan page_images baru.")
            .ro()
            .cmd("edit.edit_text")
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 } }), &["doc", "page"])),
        t("image_edit", "Edit gambar", "Ubah salah satu gambar halaman (nomor dari page_images): action move (rect: kotak penempatan penuh baru dalam poin bertitik asal kiri atas, berdasarkan page_images.rect), crop (rect: bagian yang dipertahankan, poin bertitik asal kiri atas; piksel tetap di tempat dan page_images.visible_rect menunjukkan hasil; pemindahan berikutnya membawa pangkas), rotate (perempat searah jarum jam, bawaan 1), rotate_by (degrees: sudut apa pun searah jarum jam di sekitar titik tengah yang terlihat; kotak dalam page_images lalu menjadi kotak pembatas gambar yang terputar), flip_horizontal, flip_vertical, replace (path: berkas gambar, digambar di tempat yang sama), atau delete. Dapat dibatalkan. Pangkas gambar terpilih bertransformasi bersama pikselnya, mengisolasi cakupan jalur bersama sambil mempertahankan gambar tetangga. Klip kerangka teks mengembalikan galat yang jelas. Rotasi/balik memakai titik tengah yang terlihat. Karya Form mendukung move/resize/rotate/flip/delete sebagai grup; replace dan image_save hanya mendukung gambar raster.")
            .cmd("edit.edit_text")
            .with(schema(
                json!({
                    "doc": doc(),
                    "page": { "type": "integer", "minimum": 1 },
                    "image": { "type": "integer", "minimum": 1 },
                    "action": { "type": "string", "enum": ["move", "crop", "rotate", "rotate_by", "flip_horizontal", "flip_vertical", "replace", "delete"] },
                    "rect": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 },
                    "quarters": { "type": "integer" },
                    "degrees": { "type": "number" },
                    "path": { "type": "string" },
                }),
                &["doc", "page", "image", "action"],
            )),
        t("image_save", "Simpan gambar sebagai", "Tulis salah satu gambar raster halaman (nomor dari page_images; form ditolak) ke path: gambar JPEG tidak berubah, lainnya sebagai PNG (ekstensi ditambah bila tidak ada).")
            .destructive()
            .cmd("edit.edit_text")
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "image": { "type": "integer", "minimum": 1 }, "path": { "type": "string" } }), &["doc", "page", "image", "path"])),
        t("text_font_files", "Daftar font dokumen terpasang", "Temukan berkas TrueType/OpenType berlisensi terbuka yang dapat ditanam atas permintaan. Direktori opsional (maksimal 16) membatasi pemindaian; dengan --root, hanya root itu yang dipindai sebagai bawaan dan folder yang diberikan harus tetap di dalamnya. Pemindaian melewati symlink, koleksi, dan font tidak valid/terbatas lisensi, dengan anggaran kedalaman, entri, dan byte. Pilih path hasil dengan text_font_import.")
            .ro().cmd("edit.edit_text")
            .with(schema(json!({"directories":{"type":"array","items":{"type":"string"},"maxItems":16}}), &[])),
        t("text_font_import", "Muat font dokumen", "Muat face TrueType/OpenType penuh berlisensi terbuka yang dipilih secara eksplisit ke halaman. Keluarga program, izin penanaman, dan pernyataan lisensi divalidasi. Melengkapi subset sumber face yang sama tanpa mengganti glyph yang ada. Mengembalikan resource font yang dapat dipakai dengan text_edit.font_resource. Dapat dibatalkan; memperhatikan pembatasan root. Tidak ada cadangan antarmuka sistem yang diekspor.")
            .cmd("edit.edit_text")
            .with(schema(json!({"doc":doc(), "page":{"type":"integer","minimum":1}, "path":{"type":"string"}}), &["doc","page","path"])),
        t("text_fonts", "Daftar font dokumen", "Daftar face sumber tertanam tervalidasi dan face pratinjau penuh yang disetujui untuk halaman. Resource terdaftar penuh dapat dipilih oleh text_edit.font_resource; face preview_only bukan nama resource PDF.")
            .ro().cmd("edit.edit_text")
            .with(schema(json!({"doc":doc(), "page":{"type":"integer","minimum":1}}), &["doc","page"])),
        t("text_paragraphs", "Daftar paragraf", "Paragraf pada halaman (deretan sumber berdekatan yang dikelompokkan menjadi baris visual sejajar, mencakup font, ukuran, dan warna campuran): nomor, teks, editor_text (baris visual dipisahkan baris baru), baris dan nomor baris sumber, kotak (poin bertitik asal kiri atas), font, dan ukuran. Gunakan nomor dengan paragraph text_edit: sampai halaman dicantumkan lagi, nomor menamai paragraf yang dicantumkan di sini, bahkan sesudah edit mengelompokkan ulang halaman.")
            .ro()
            .cmd("edit.edit_text")
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 } }), &["doc", "page"])),
        t("text_edit_layout", "Ukur draf teks", "Ukur draf paragraf memakai penulisan ulang PDF yang sama dengan Apply, tanpa mengubah dokumen. Mengembalikan source_rect, draft_rect (poin bertitik asal kiri atas), overflow, dan face sumber sfnt tertanam yang tersedia untuk editor sebaris. Teks bawaan paragraf yang tidak berubah. size, width, font_resource, dan ranges UTF-8 terpisah opsional mempratinjau perubahan format.")
            .ro()
            .cmd("edit.edit_text")
            .with(schema(json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "paragraph": { "type": "integer", "minimum": 1 }, "text": { "type": "string" }, "size": { "type": "number" }, "width": { "type": "number" }, "font_resource":{"type":"string"}, "ranges":{"type":"array","maxItems":1024,"items":{"type":"object"}} }), &["doc", "page", "paragraph"])),
        t("text_edit", "Edit teks", "Ganti teks satu paragraf (paragraph, dari text_paragraphs: dibungkus ulang ke lebar paragraf dengan jarak barisnya) atau satu baris (line, dari text_lines) di tempat, mempertahankan posisi, ukuran, dan warna. Untuk wilayah gaya campuran, editor_text memberikan pemisah baris visual; teks yang bertahan mempertahankan gaya deretan sumbernya dan teks sisipan mewarisi deretan di sekitarnya. Untuk paragraf, ubah juga formatnya: font (helvetica, times, courier) dengan bold/italic, size (poin), color (#rrggbb), align (left, center, right, justify), underline, line_spacing (× size), char_spacing (poin), dan scale (horizontal, persen), atau pindahkan (dx, dy dalam poin; ke atas adalah +dy) dan bungkus ulang ke lebar baru (poin); text lalu dapat dikosongkan. Fontnya sendiri dipakai ulang bila dapat menampilkan tiap karakter dan tidak ada perubahan font/bobot yang diminta; jika tidak, memakai font standar yang cocok dengan gaya sumber atau, untuk teks yang tidak dapat ditampilkan font standar mana pun, kerangka cadangan nyata dari face milik PdfCraft, dari yang memiliki tiap karakter: face Jepang craft-fonts (Mincho/Gothic yang cocok dengan keluarga yang diminta/sumber dan bobot yang tersedia) atau, di belakangnya, face Inter bawaan, yang memuat huruf Latin, Yunani, dan Kiril yang tidak dimiliki keduanya (қ Uzbek, ә Kazakh). Hasil menunjukkan font yang dipakai, dan untuk paragraf teksnya kecuali keamanan dokumen tidak mengizinkan penyalinan. Bobot Jepang yang hilang memakai Regular, bukan tebal sintetis; build web kecil hanya memiliki Gothic Regular. Tebal paragraf dapat diatur tanpa font untuk mempertahankan keluarga sumber. Teks yang tidak dapat ditampilkan font yang tersedia ditolak. Nomor paragraf ditolak bila edit sebelumnya menggabungkan paragraf itu ke tetangga (cantumkan halaman lagi); stale_paragraphs hasil menamai nomor yang hilang. Dapat dibatalkan.")
            .cmd("edit.edit_text")
            .with(schema(
                json!({ "doc": doc(), "page": { "type": "integer", "minimum": 1 }, "line": { "type": "integer", "minimum": 1 },
                    "paragraph": { "type": "integer", "minimum": 1 },
                    "font_resource": { "type": "string", "description": "Resource font sumber/halaman penuh eksplisit dari text_fonts atau text_font_import." },
                    "ranges": { "type": "array", "maxItems": 1024, "items": { "type": "object", "properties": {"range":{"type":"array","items":{"type":"integer","minimum":0},"minItems":2,"maxItems":2},"font":{"type":"string","enum":["helvetica","times","courier"]},"bold":{"type":"boolean"},"italic":{"type":"boolean"},"font_resource":{"type":"string"},"size":{"type":"number","exclusiveMinimum":0},"color":{"type":"string"},"underline":{"type":"boolean"},"char_spacing":{"type":"number"},"scale":{"type":"number"}},"required":["range"]}, "description":"Rentang byte UTF-8 terpisah dalam teks pengganti; format sumber yang tidak ditentukan bertahan." },
                    "text": { "type": "string" },
                    "font": { "type": "string", "enum": ["helvetica", "times", "courier"] },
                    "bold": { "type": "boolean" },
                    "italic": { "type": "boolean" },
                    "size": { "type": "number", "exclusiveMinimum": 0 },
                    "color": { "type": "string", "description": "#rrggbb" },
                    "align": { "type": "string", "enum": ["left", "center", "right", "justify"] },
                    "underline": { "type": "boolean" },
                    "line_spacing": { "type": "number", "description": "Kelipatan ukuran font (1.2 adalah biasa)." },
                    "dx": { "type": "number", "description": "Khusus paragraf: geser ke kanan sebesar poin ini (negatif: kiri)." },
                    "dy": { "type": "number", "description": "Khusus paragraf: geser ke atas sebesar poin ini (negatif: bawah)." },
                    "width": { "type": "number", "description": "Khusus paragraf: bungkus ulang ke lebar ini dalam poin (menyeret tepi kotak)." },
                    "char_spacing": { "type": "number", "description": "Points." },
                    "scale": { "type": "number", "description": "Skala horizontal dalam persen." } }),
                &["doc", "page"],
            )),
        t("doc_revisions", "Daftar revisi", "Daftar revisi tersimpan dokumen (terlama lebih dulu): tiap pembaruan inkremental adalah satu. Mengembalikan nomor revisi, tempat berakhirnya dalam berkas dan ukurannya, dan tanda tangan mana yang menandatangani tepat revisi itu.")
            .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("doc_open_revision", "Buka revisi", "Buka revisi tersimpan `revision` (1 = terlama) dari dokumen sebagai dokumen baru yang belum disimpan, untuk melihat berkas seperti saat itu.")
            .with(schema(json!({ "doc": doc(), "revision": { "type": "integer", "minimum": 1 } }), &["doc", "revision"])),
        t("doc_optimize", "Optimalkan PDF", "Tulis salinan teroptimalkan ke `path` (PDF Optimizer Acrobat). preset: standard (bawaan; pilihan Reduce File Size) atau mobile (gambar ke 96 ppi di atas 144, JPEG kualitas tinggi); opsi di bawah mengubahnya. compatibility: retain (bawaan; mempertahankan versi dokumen, atau 1.5 bila stream objek ditulis), 1.4, 1.5, 1.6, 1.7, atau 2.0. merge_fragments (bawaan true) lebih dulu menyatukan kembali gambar halaman yang digambar sebagai banyak strip gambar bertumpuk (seperti yang ditulis sebagian printer PDF) menjadi satu gambar, agar dapat di-resample (merged_images, fragments_merged). color / gray: { downsample, ppi, above_ppi, compression: jpeg|zip|retain, quality 1–100 } (bawaan: downsample ke 150 ppi di atas 225, JPEG 60). Gambar diukur tempat halaman menggambarnya dan hanya diganti bila lebih kecil. discard_*: thumbnails (bawaan true), alternate_images (true), tags, print_settings; flate_unencoded (true); object_streams (bawaan true; selalu nonaktif untuk 1.4); lzw_to_flate (true) mendekode stream LZW dan menyimpannya dengan Flate sebagai gantinya, meninggalkan yang tidak dapat didekode tidak berubah (lzw_streams menghitung stream yang dikonversi); remove_invalid_links dan remove_unreferenced_dests (true); gambar dan form yang dicantumkan halaman tetapi tidak pernah digambar selalu dibuang (unused_xobjects). discard: kategori Remove Hidden Information (metadata, attachments, comments, form-fields, hidden-text, hidden-layers, bookmarks, links-actions-scripts, private-data). Dokumen bertanda tangan ditolak. Dokumen terbuka tidak berubah.")
            .cmd("optimize.advanced")
            .with(schema(
                json!({
                    "doc": doc(),
                    "path": { "type": "string" },
                    "preset": { "type": "string", "enum": ["standard", "mobile"] },
                    "compatibility": { "type": "string", "enum": ["retain", "1.4", "1.5", "1.6", "1.7", "2.0"] },
                    "merge_fragments": { "type": "boolean" },
                    "color": { "type": "object" },
                    "gray": { "type": "object" },
                    "discard_thumbnails": { "type": "boolean" },
                    "discard_alternate_images": { "type": "boolean" },
                    "discard_tags": { "type": "boolean" },
                    "discard_print_settings": { "type": "boolean" },
                    "flate_unencoded": { "type": "boolean" },
                    "object_streams": { "type": "boolean" },
                    "lzw_to_flate": { "type": "boolean" },
                    "remove_invalid_links": { "type": "boolean" },
                    "remove_unreferenced_dests": { "type": "boolean" },
                    "discard": { "type": "array", "items": { "type": "string" } },
                }),
                &["doc", "path"],
            )),
        t("doc_flatten", "Ratakan", "Gabungkan tampilan komentar dan/atau bidang formulir ke konten halaman, sehingga tercetak dan tampil di mana pun tetapi tidak lagi dapat diedit. Perataan bidang formulir XFA juga mengeluarkan XFA (template, data, dan skripnya), sehingga penampil Adobe menampilkan halaman yang sudah dibakar daripada menata formulir lagi. Dapat dibatalkan sampai disimpan.")
            .destructive()
            .with(schema(json!({ "doc": doc(), "comments": { "type": "boolean", "description": "Bawaan true." }, "fields": { "type": "boolean", "description": "Bawaan true." } }), &["doc"])),
        t(
            "xfa_to_static",
            "Ubah formulir XFA menjadi formulir statis",
            "Jadikan formulir XFA formulir PDF biasa: paket XFA (template, data, skrip) dan kunci privat PdfCraft hilang; halaman seperti tertata dan bidang beserta nilainya tetap, sehingga setiap penampil menampilkan formulir yang sama dan tidak ada yang menatanya lagi. Skripnya tidak lagi berjalan. Dapat dibatalkan sampai disimpan.",
        )
        .destructive()
        .with(schema(json!({ "doc": doc() }), &["doc"])),
        t("edit_undo", "Batalkan", "Batalkan edit terakhir dokumen.").cmd("edit.undo").with(schema(json!({ "doc": doc() }), &["doc"])),
        t("edit_redo", "Ulangi", "Ulangi edit dokumen terakhir yang dibatalkan.").cmd("edit.redo").with(schema(json!({ "doc": doc() }), &["doc"])),
        t("command_list", "Daftar perintah", "Setiap perintah PdfCraft terdaftar beserta menu, pintasan, apakah aktif sekarang, dan tool yang mengotomatiskannya.")
            .ro()
            .with(schema(json!({ "doc": doc(), "filter":{"type":"string"}, "enabled_only":{"type":"boolean"} }), &[])),
        t("command_run", "Jalankan perintah", "Jalankan perintah registry melalui tool headless-nya. Isi argumen tool itu dalam params (lihat command_list). Parameter tidak dikenal diabaikan dengan peringatan; pembatasan path tetap berlaku.")
            .destructive().with(schema(json!({"id":{"type":"string"},"params":{"type":"object"}}), &["id"])),
        t("command_batch", "Jalankan beberapa perintah", "Jalankan langkah secara berurutan (maksimal 1000). Mengembalikan jumlah selesai/gagal dan ok/result/error per langkah; stop_on_error bawaan true.")
            .destructive().with(schema(json!({"steps":{"type":"array","items":schema(json!({"id":{"type":"string"},"params":{"type":"object"}}), &["id"])},"stop_on_error":{"type":"boolean"}}), &["steps"])),
        t("doc_inspect", "Periksa dokumen terbuka", "Dengan doc, kembalikan doc_info; jika tidak, kembalikan informasi setiap dokumen terbuka (array documents kosong bila tidak ada yang terbuka).")
            .ro().with(schema(json!({"doc":doc()}), &[])),
        t("render_preview", "Render pratinjau", "Render PNG yang dibatasi max_side (bawaan 1024), tanpa mengubah atau menyimpan dokumen. Page bawaan 1; doc dapat dikosongkan bila tepat satu dokumen terbuka.")
            .ro().with(schema(json!({"doc":doc(),"page":{"type":"integer","minimum":1},"max_side":{"type":"integer","minimum":1,"maximum":4096}}), &[])),
    ]
}

static TOOLS: std::sync::LazyLock<Vec<ToolDef>> = std::sync::LazyLock::new(tools);

pub(crate) fn find(name: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.name == name)
}

pub(crate) fn tool_for_command(id: &str) -> Option<&'static str> {
    TOOLS.iter().find(|t| t.command == Some(id)).map(|t| t.name)
}

/// Reject unknown and missing arguments (types are checked when each value is read).
pub(crate) fn check_args(def: &ToolDef, args: &Value) -> Result<(), ToolError> {
    let obj = args.as_object().ok_or_else(|| ToolError::InvalidArgs("arguments must be a JSON object".into()))?;
    let props = def.input_schema["properties"].as_object().cloned().unwrap_or_default();
    if let Some(k) = obj.keys().find(|k| !props.contains_key(*k)) {
        let known: Vec<&str> = props.keys().map(String::as_str).collect();
        return Err(ToolError::InvalidArgs(format!("{}: unknown argument {k:?} (expected: {})", def.name, known.join(", "))));
    }
    for r in def.input_schema["required"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        if obj.get(r).is_none_or(Value::is_null) {
            return Err(ToolError::InvalidArgs(format!("{}: missing argument {r}", def.name)));
        }
    }
    Ok(())
}
