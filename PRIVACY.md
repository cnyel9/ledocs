# Kebijakan Privasi

**Tanggal berlaku:** 14 November 2026

**Pengembang:** Levi Setiadi

LeDocs adalah aplikasi dokumen, catatan, dan foto yang bekerja di perangkat Anda. Offline secara bawaan: online hanya atas perintah Anda, lewat satu fitur opsional di bawah ini.

## Apa yang dikumpulkan LeDocs

Tidak ada. LeDocs tidak memiliki sistem akun, iklan, analitik, maupun pelaporan kerusakan. Kami tidak menjalankan server yang menerima berkas, catatan, rekaman, foto, atau apa pun tentang cara Anda memakai aplikasi ini. Fitur online opsional di bawah mengirim data ke layanan pihak ketiga yang Anda pilih, bukan ke kami.

## Akses jaringan

Setiap dokumen (PDF, Word, Excel, PowerPoint), catatan, daftar, memo suara, dan foto dibuka, disunting, dan disimpan di perangkat Anda oleh mesin LeDocs sendiri. Pengenalan teks (OCR) berjalan di perangkat Anda dengan model yang disertakan di dalam aplikasi. Semua fitur dokumen bekerja sepenuhnya tanpa internet, termasuk dalam mode pesawat.

Hanya ada dua pemakai jaringan, keduanya di bawah kendali Anda. Pertama adalah **Asisten AI**, fitur opsional yang **mati secara bawaan** dan hanya bekerja setelah Anda menyalakannya di Pengaturan:

- Bila Anda bertanya kepada Asisten AI, teks pertanyaan Anda dan, bila Anda melampirkan dokumen yang sedang terbuka, teks hasil ekstraksi dokumen itu dikirim **langsung dari perangkat Anda ke provider yang Anda pilih** (host yang tertulis di pengaturan provider, misalnya openrouter.ai, api.deepseek.com, atau server Anda sendiri). Tidak ada server perantara milik pengembang.
- API key Anda tersimpan terenkripsi di perangkat dengan Android Keystore, hanya dikirim ke provider pemiliknya, tidak pernah ke pengembang, dan tidak pernah ditulis ke log.
- Sebelum pengiriman pertama yang melampirkan dokumen ke sebuah provider, aplikasi menampilkan tujuan dan isi yang akan dikirim, dan meminta persetujuan Anda. Menyalakan Asisten AI tidak mengirim apa pun dengan sendirinya.
- Mematikan Asisten AI menghentikan seluruh lalu lintas fitur ini. Riwayat obrolan tersimpan hanya di perangkat Anda.

Kedua adalah **pemeriksa pembaruan**: saat aplikasi dibuka, LeDocs menanyakan rilis terakhir ke GitHub (api.github.com) untuk melihat apakah ada versi yang lebih baru, dan mengunduh APK rilis hanya setelah Anda mengetuk Unduh & Pasang. Pemeriksaan itu tidak mengirim data pribadi atau dokumen apa pun, dan APK yang diunduh diverifikasi ukuran, hash SHA-256, dan sertifikat penandatangannya sebelum dipasang. Di luar dua hal ini, aplikasi tidak melakukan panggilan jaringan apa pun: tidak ada sinkronisasi atau telemetri.

## Berkas dan izin

LeDocs hanya membaca berkas yang Anda buka atau folder yang Anda pilih untuk ditampilkan, dan menyimpan salinan kerja di penyimpanan privatnya sendiri supaya perubahan Anda bisa disimpan. Mikrofon hanya dipakai selama Anda merekam memo suara. Pemindaian memakai aplikasi kamera sistem. Berkas yang Anda bagikan atau cetak diserahkan ke aplikasi pilihan Anda melalui Android.

## Anak-anak

LeDocs tidak mengumpulkan data pribadi dari siapa pun, termasuk anak-anak.

## Perubahan

Bila versi mendatang mengubah isi halaman ini, kami akan memperbarui halaman ini beserta tanggal berlakunya, dan menyatakannya di catatan rilis.

## Kontak

Pertanyaan atau kekhawatiran privasi: buka issue di [repo LeDocs](https://github.com/cnyel9/ledocs/issues). Untuk kerentanan keamanan, gunakan jalur pelaporan privat di [SECURITY.md](SECURITY.md), bukan issue publik.
