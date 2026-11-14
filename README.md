# LeDocs

### Pembaca dan penyunting PDF serta dokumen Office untuk Android. Gratis, tanpa iklan, tanpa akun. Offline secara bawaan, online hanya atas perintah Anda.

Baca, tanda tangani, dan sunting PDF, buka dan sunting berkas Word dan Excel, tampilkan PowerPoint, serta simpan catatan, daftar tugas, foto, dan memo suara dalam satu tempat. Semua fitur dokumen berjalan sepenuhnya di perangkat Anda, tanpa pelacak dan tanpa sistem akun. Dokumen Anda tidak pernah meninggalkan perangkat Anda, kecuali teks yang Anda sendiri kirimkan ke Asisten AI bila Anda menyalakan fitur opsional itu.

Antarmuka LeDocs sepenuhnya berbahasa Indonesia dengan latar gelap hitam.

## Yang bisa dilakukan

### Baca dan olah PDF

- Baca dengan gulir halus, cubit untuk zoom, gambar mini halaman, markah, mode baca, dan pencarian di dalam dokumen.
- Pilih dan salin teks, bacakan halaman dengan suara, jalankan OCR di perangkat untuk halaman hasil pindai.
- Sorot, garis bawah, coret, gambar tinta, tambah bentuk, catatan teks dan gambar, sunting teks asli PDF.
- Isi formulir, tanda tangani dengan gambar tangan, ketikan, atau sertifikat (ID digital), dan verifikasi tanda tangan.
- Atur, putar, pangkas, hapus, dan ekstrak halaman, gabung, pisah, kompres, lindungi (AES-256), dan ekspor PDF, buat PDF dari foto, teks, atau pindaian.

### Buka dan sunting dokumen Office

- **Word** (.docx, .doc, .odt, .rtf, Markdown, TXT): baca, cari, bacakan, sunting paragraf di tempat dengan tebal, miring, daftar, perataan, dan judul. Simpan sebagai Word, OpenDocument, RTF, HTML, Markdown, TXT, atau PDF.
- **Excel** (.xlsx, .xls, .xlsb, .ods, .csv): kisi yang bisa digeser dengan panel beku dan tab lembar, nilai dan rumus yang menghitung ulang, format sel, sisip baris dan kolom, cari, dan urungkan. Simpan sebagai Excel atau CSV, atau ekspor ke PDF.
- **PowerPoint** (.pptx, dan teks dari .ppt): slide sebagai halaman, mode presentasi layar penuh, pencarian, bacakan, dan ekspor ke PDF.

### Catatan, daftar, foto, dan suara

- Catatan cepat dengan format ringan, daftar tugas dan belanja, memo suara dengan gelombang bunyi, dan catatan foto.
- Penyunting gambar dengan paparan, kontras, sorotan, bayangan, kecerahan warna, kejenuhan, dan putar.
- Pratinjau Cepat: buka berkas dari WhatsApp, Gmail, atau pengelola berkas dan berkas itu melayang di atas aplikasi tersebut sebagai kartu kaca, seperti Quick Look di Android.

## Versi Android yang didukung

Android 10 (API 29) atau yang lebih baru.

## Unduh

APK rilis diterbitkan di halaman [Releases](https://github.com/cnyel9/ledocs/releases). Pilih APK yang sesuai dengan perangkat Anda:

| APK | Untuk |
| --- | --- |
| `LeDocs-v<versi>.apk` (arm64-v8a) | Sebagian besar ponsel dan tablet ARM 64-bit saat ini |

## Teknologi

- **Mesin:** Rust, satu library native (PDF, Word, PowerPoint, Excel, dan gambar), disambungkan ke aplikasi dengan UniFFI. OCR memakai mesin Ocrs di perangkat dengan model yang disertakan.
- **Aplikasi:** Kotlin dan Jetpack Compose, Material 3 Expressive.
- **Bahasa:** Bahasa Indonesia.

## Bangun dari sumber

Kebutuhan:

- JDK 21 (CI) atau JDK 17+ secara lokal, Android SDK Platform 37 dan Build Tools, Android NDK 29.0.14206865.
- Rust (toolchain tertera di `engine/rust-toolchain.toml`) dengan cargo-ndk 4.1.2.
- Gradle 9.8.2.

```bash
git clone https://github.com/cnyel9/ledocs.git
cd ledocs/android
./gradlew assembleRelease -Pledocs.abis=arm64-v8a
```

Build rilis ditandatangani dengan kredensial lingkungan `LEDOCS_UPLOAD_*` bila diatur, selain itu dengan kunci debug. Jangan sebarkan build bertanda tangan debug sebagai rilis resmi.

Jalankan tes mesin:

```bash
cd engine
cargo test -p artefact-mobile
```

Struktur proyek: `android/` aplikasi, `engine/` mesin Rust, `fastlane/` teks etalase, `tools/` perkakas pengembang.

## Privasi

Izin `INTERNET` hanya dipakai Asisten AI, fitur opsional yang mati secara bawaan: pertanyaan dan teks dokumen yang Anda lampirkan dikirim langsung dari perangkat Anda ke provider pilihan Anda, tidak pernah lewat server pengembang. Di luar fitur itu aplikasi tidak melakukan panggilan jaringan apa pun. LeDocs juga tidak menyatakan izin penyimpanan atau kamera: berkas datang lewat pemilih sistem dan aplikasi kamera sistem. Lihat `PRIVACY.md`.

## Lisensi

MIT ATAU Apache-2.0, sesuai pilihan Anda. Lihat `LICENSE-APACHE`, `LICENSE-MIT`, `NOTICE`, dan `ATTRIBUTION.md`.
