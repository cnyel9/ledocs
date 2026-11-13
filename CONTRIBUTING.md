# Berkontribusi ke LeDocs

Terima kasih sudah membantu! Laporan bug, ide, dan pull request yang fokus semuanya diterima.

## Sebelum mulai

- Lihat dulu [issues](https://github.com/cnyel9/ledocs/issues) yang sudah ada; untuk perubahan besar, buka issue dulu supaya pendekatannya bisa disepakati.
- Satu pull request untuk satu perubahan, dengan deskripsi yang jelas dan tangkapan layar atau video singkat untuk perubahan yang terlihat.

## Membangun

Lihat [Bangun dari sumber](README.md#bangun-dari-sumber). Aplikasi ada di `android/` (Kotlin, Jetpack Compose); mesin ada di `engine/` (Rust). Jalankan `cargo test -p artefact-mobile` di `engine/` sesudah mengubah mesin, dan perbaiki setiap peringatan (`cargo clippy`).

## Gaya

- Ikuti kode di sekitar perubahanmu: penamaan, kepadatan komentar, dan idiom.
- UI memakai komponen di `android/app/src/main/java/com/ledocs/app/ui/glass/` dan teks di `res/values/strings.xml`.
- Setiap aset atau library pihak ketiga dicantumkan di `ATTRIBUTION.toml` / `ATTRIBUTION.md` dan halaman lisensi aplikasi.

## Lisensi

Dengan berkontribusi, kamu setuju kontribusimu dilisensikan di bawah MIT ATAU Apache-2.0, sama seperti proyek ini.
