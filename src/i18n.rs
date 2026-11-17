//! A tiny, dependency-free internationalisation layer.
//!
//! UI strings are written in English at the call site and wrapped in
//! [`tr!`](crate::tr). The lookup table maps the English source string to a
//! translation for the current language, so any string that has no translation
//! simply falls back to English — a missing entry can never break the UI.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Tr,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Tr => "tr",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_ascii_lowercase().as_str() {
            "tr" | "tr-tr" | "turkish" | "türkçe" => Lang::Tr,
            _ => Lang::En,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Tr => "Türkçe",
        }
    }

    pub const ALL: [Lang; 2] = [Lang::En, Lang::Tr];
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::En => 0,
            Lang::Tr => 1,
        } as u8,
        Ordering::Relaxed,
    );
}

pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Tr,
        _ => Lang::En,
    }
}

/// The Turkish translation table: `(English source, Turkish)`.
///
/// Keys are matched exactly against the English string used in the UI.
const TR: &[(&str, &str)] = &[
    // ---------------------------------------------------------- commands
    ("New note", "Yeni not"),
    (
        "New sub-note (under current note)",
        "Yeni alt not (açık notun altında)",
    ),
    ("Open vault", "Kasa aç"),
    ("Open today's daily note", "Bugünün günlük notunu aç"),
    ("Open previous daily note", "Önceki günlük notu aç"),
    ("Open next daily note", "Sonraki günlük notu aç"),
    ("Daily notes settings", "Günlük not ayarları"),
    ("New note from template", "Şablondan yeni not"),
    ("New canvas", "Yeni canvas"),
    ("Open canvas board", "Canvas panosu aç"),
    ("RSS feeds", "RSS akışları"),
    ("Password manager", "Şifre yöneticisi"),
    ("Undo", "Geri al"),
    ("Redo", "Yinele"),
    ("Cycle window mode (F11)", "Pencere modunu değiştir (F11)"),
    ("Toggle Vim mode", "Vim modunu aç/kapat"),
    (
        "Cycle editor view (edit/split/preview)",
        "Editör görünümünü değiştir (düzenle/böl/önizle)",
    ),
    (
        "Toggle concealed markdown (WYSIWYG editing)",
        "Gizlenen markdown (WYSIWYG) aç/kapat",
    ),
    ("Toggle spell check", "Yazım denetimini aç/kapat"),
    (
        "Toggle second note pane (split view)",
        "İkinci not panelini aç/kapat (bölünmüş görünüm)",
    ),
    (
        "Find / replace in current note (Ctrl+F)",
        "Geçerli notta bul/değiştir (Ctrl+F)",
    ),
    (
        "Column edit / multi-cursor insert",
        "Sütun düzenleme / çoklu imleç ekleme",
    ),
    (
        "Edit note properties (frontmatter)",
        "Not özelliklerini düzenle (frontmatter)",
    ),
    (
        "Manage tags (rename / merge / delete)",
        "Etiketleri yönet (yeniden adlandır / birleştir / sil)",
    ),
    ("Edit table visually", "Tabloyu görsel olarak düzenle"),
    (
        "Copy block reference for the current line",
        "Geçerli satır için blok referansı kopyala",
    ),
    ("Settings", "Ayarlar"),
    ("Open a random note", "Rastgele not aç"),
    (
        "Bookmark / unbookmark current note",
        "Notu yer imine ekle / çıkar",
    ),
    (
        "Note composer (merge / split)…",
        "Not birleştirici (birleştir / böl)…",
    ),
    ("Record a voice note", "Ses notu kaydet"),
    (
        "Toggle linked second pane",
        "Bağlantılı ikinci paneli aç/kapat",
    ),
    ("Toggle side panel", "Yan paneli aç/kapat"),
    ("Toggle graph view", "Graf görünümünü aç/kapat"),
    ("Lock the vault", "Kasayı kilitle"),
    ("Close current tab", "Geçerli sekmeyi kapat"),
    ("Quit", "Çık"),
    // ------------------------------------------------------- palette / UI
    ("Command palette", "Komut paleti"),
    ("Command palette (Ctrl+P)", "Komut paleti (Ctrl+P)"),
    ("Type a note or command…", "Bir not veya komut yaz…"),
    ("No matches", "Eşleşme yok"),
    ("Attach a {}", "Bir {} ekle"),
    ("Attach file", "Dosya ekle"),
    ("Attach:", "Ekle:"),
    ("Attach an image", "Görsel ekle"),
    ("Attach a video", "Video ekle"),
    (
        "Attach a voice recording / audio",
        "Ses kaydı / ses dosyası ekle",
    ),
    (
        "images, voice notes, videos — shown & played in the preview",
        "görseller, ses notları, videolar — önizlemede gösterilir ve oynatılır",
    ),
    ("Font size", "Yazı boyutu"),
    ("Editor", "Editör"),
    ("Daily notes", "Günlük notlar"),
    ("Folder", "Klasör"),
    ("Date format", "Tarih biçimi"),
    ("Template", "Şablon"),
    ("Templates folder", "Şablon klasörü"),
    ("Startup window", "Başlangıç penceresi"),
    ("Window", "Pencere"),
    ("Save", "Kaydet"),
    ("Save settings", "Ayarları kaydet"),
    ("Close", "Kapat"),
    ("Fullscreen", "Tam ekran"),
    ("Maximized", "Büyütülmüş"),
    ("Windowed", "Pencereli"),
    ("image", "görsel"),
    ("video", "video"),
    ("voice recording / audio", "ses kaydı / ses"),
    ("PDF", "PDF"),
    ("Cancel", "İptal"),
    ("Apply", "Uygula"),
    ("Delete", "Sil"),
    ("Rename", "Yeniden adlandır"),
    ("Add", "Ekle"),
    ("Edit", "Düzenle"),
    ("Copy", "Kopyala"),
    ("Search", "Ara"),
    ("Refresh vault", "Kasayı yenile"),
    ("Bookmark this note", "Bu notu yer imine ekle"),
    ("Remove bookmark", "Yer imini kaldır"),
    ("Bookmarks ({})", "Yer imleri ({})"),
    (
        "Star a note to bookmark it",
        "Yer imine eklemek için bir notu yıldızla",
    ),
    ("Random bookmark", "Rastgele yer imi"),
    ("Calendar", "Takvim"),
    ("Month", "Ay"),
    ("TODAY", "BUGÜN"),
    ("Tags", "Etiketler"),
    ("No tags yet", "Henüz etiket yok"),
    ("Outline", "Ana hat"),
    ("No headings", "Başlık yok"),
    ("Outgoing links", "Giden bağlantılar"),
    ("No [[wikilinks]]", "[[wikilink]] yok"),
    ("Backlinks ({count})", "Geri bağlantılar ({count})"),
    (
        "No notes link here yet",
        "Buraya henüz bağlantı veren not yok",
    ),
    (
        "Unlinked mentions ({ucount})",
        "Bağlantısız anmalar ({ucount})",
    ),
    ("No unlinked mentions", "Bağlantısız anma yok"),
    ("Properties ({})", "Özellikler ({})"),
    ("No frontmatter properties", "Frontmatter özelliği yok"),
    ("Add / edit properties", "Özellik ekle / düzenle"),
    (
        "Select a note from the sidebar",
        "Kenar çubuğundan bir not seç",
    ),
    ("No notes to graph", "Grafiklenecek not yok"),
    ("Graph", "Graf"),
    (
        "Local graph (near the active note)",
        "Yerel graf (açık notun çevresi)",
    ),
    ("Show notes with no links", "Bağlantısız notları göster"),
    ("tag filter", "etiket filtresi"),
    ("Colors", "Renkler"),
    ("Force", "Kuvvet"),
    ("Repulsion", "İtme"),
    ("Attraction", "Çekme"),
    ("Recompute layout", "Yerleşimi yeniden hesapla"),
    ("Colour notes by tag", "Notları etikete göre renklendir"),
    ("Reset view", "Görünümü sıfırla"),
    ("Close graph", "Grafları kapat"),
    ("Shallower", "Daha sığ"),
    ("Deeper", "Daha derin"),
    ("Words", "Kelime"),
    ("Find in note", "Notta bul"),
    ("Replace with", "Şununla değiştir"),
    ("Replace", "Değiştir"),
    ("Replace all", "Tümünü değiştir"),
    ("Match case", "Büyük/küçük harf eşleşsin"),
    (
        "Previous match (Shift+Enter)",
        "Önceki eşleşme (Shift+Enter)",
    ),
    ("Next match (Enter)", "Sonraki eşleşme (Enter)"),
    ("Close find bar", "Bulma çubuğunu kapat"),
    ("no matches", "eşleşme yok"),
    // ------------------------------------------------------------ dialogs
    ("Open vault", "Kasa aç"),
    (
        "Choose a folder to use as your vault",
        "Kasa olarak kullanılacak klasörü seç",
    ),
    ("Go to path", "Yola git"),
    ("New note", "Yeni not"),
    ("Note name", "Not adı"),
    ("Enter a note name", "Bir not adı gir"),
    ("Create note", "Not oluştur"),
    ("New sub-note", "Yeni alt not"),
    ("Enter a sub-note name", "Bir alt not adı gir"),
    ("Create sub-note", "Alt not oluştur"),
    ("Rename note", "Notu yeniden adlandır"),
    ("Delete note", "Notu sil"),
    (
        "Delete this note permanently?",
        "Bu not kalıcı olarak silinsin mi?",
    ),
    ("Properties", "Özellikler"),
    (
        "YAML frontmatter properties for this note",
        "Bu notun YAML frontmatter özellikleri",
    ),
    (
        "No properties yet — add one below.",
        "Henüz özellik yok — aşağıdan ekle.",
    ),
    ("Add / update a property", "Özellik ekle / güncelle"),
    ("List (comma-separated)", "Liste (virgülle ayrılmış)"),
    ("Save property", "Özelliği kaydet"),
    ("Delete property", "Özelliği sil"),
    ("Edit table", "Tabloyu düzenle"),
    (
        "First row is the header. Edit cells directly.",
        "İlk satır başlıktır. Hücreleri doğrudan düzenle.",
    ),
    ("+ Row", "+ Satır"),
    ("+ Column", "+ Sütun"),
    ("Delete row:", "Satır sil:"),
    ("Delete column:", "Sütun sil:"),
    ("Save table", "Tabloyu kaydet"),
    ("Manage tags", "Etiketleri yönet"),
    (
        "Renaming rewrites the tag in every note (frontmatter and inline).",
        "Yeniden adlandırma etiketi her notta günceller (frontmatter ve satır içi).",
    ),
    ("No tags in this vault", "Bu kasada etiket yok"),
    ("Rename tag", "Etiketi yeniden adlandır"),
    ("Delete tag", "Etiketi sil"),
    ("Column edit", "Sütun düzenleme"),
    (
        "Insert the same text on every line in a range (multi-cursor workhorse).",
        "Aralıktaki her satıra aynı metni ekle (çoklu imleç yardımcısı).",
    ),
    ("From line", "Başlangıç satırı"),
    ("to", "bitiş"),
    ("At column", "Sütunda"),
    ("At end of line", "Satır sonunda"),
    ("Text", "Metin"),
    ("text to insert", "eklenecek metin"),
    ("Note composer", "Not birleştirici"),
    ("Active note: {title}", "Açık not: {title}"),
    ("Merge into another note", "Başka bir notla birleştir"),
    ("Split below the caret", "İmlecin altından böl"),
    ("target note title", "hedef not başlığı"),
    (
        "new note title (optional)",
        "yeni not başlığı (isteğe bağlı)",
    ),
    (
        "The active note's content is appended to the target and the original file is removed.",
        "Açık notun içeriği hedefe eklenir ve orijinal dosya silinir.",
    ),
    (
        "The content below the caret is moved into a new note.",
        "İmlecin altındaki içerik yeni bir nota taşınır.",
    ),
    ("Settings", "Ayarlar"),
    (
        "Conceal markdown while editing (WYSIWYG)",
        "Düzenlerken markdown'ı gizle (WYSIWYG)",
    ),
    ("Spell check", "Yazım denetimi"),
    ("Vim mode", "Vim modu"),
    (
        "Open notes in split view by default",
        "Notları varsayılan olarak bölünmüş görünümde aç",
    ),
    ("Linked second pane", "Bağlantılı ikinci panel"),
    ("Language", "Dil"),
    ("Canvas name", "Canvas adı"),
    ("Create canvas", "Canvas oluştur"),
    ("Create folder", "Klasör oluştur"),
    ("New folder", "Yeni klasör"),
    ("Delete folder", "Klasörü sil"),
    ("Category (optional)", "Kategori (isteğe bağlı)"),
    ("Add feed", "Akış ekle"),
    ("Delete feed", "Akışı sil"),
    ("Edit feed", "Akışı düzenle"),
    ("Feed", "Akış"),
    (
        "No feeds yet. Use ＋ to add one.",
        "Henüz akış yok. Eklemek için ＋ kullan.",
    ),
    ("Uncategorized", "Kategorisiz"),
    ("Saved", "Kaydedilenler"),
    (
        "No saved articles yet — press the bookmark icon on an article.",
        "Henüz kaydedilmiş makale yok — bir makalede yer imi simgesine bas.",
    ),
    ("Load full article", "Tam makaleyi yükle"),
    ("Open in browser", "Tarayıcıda aç"),
    // ------------------------------------------------------------- gate
    ("Encrypted Note", "Encrypted Note"),
    (
        "AES-256 encrypted markdown vault",
        "AES-256 şifreli markdown kasası",
    ),
    ("Open a vault", "Bir kasa aç"),
    ("Recent vaults", "Son kasalar"),
    ("Create a master password", "Ana şifre oluştur"),
    ("Enter your master password", "Ana şifreni gir"),
    ("Master password", "Ana şifre"),
    ("Confirm password", "Şifreyi doğrula"),
    ("Unlock", "Kilidi aç"),
    ("Locked", "Kilitli"),
    ("Forgot password?", "Şifreyi mi unuttun?"),
    ("Record your master password", "Ana şifreni kaydet"),
    ("At least 16 characters", "En az 16 karakter"),
    ("A number", "Bir rakam"),
    ("A capital letter", "Bir büyük harf"),
    ("A lower-case letter", "Bir küçük harf"),
    ("A special character", "Bir özel karakter"),
    ("No words", "Kelime yok"),
    (
        "No words (max 3 letters in a row)",
        "Kelime yok (üst üste en fazla 3 harf)",
    ),
    (
        "No articles — press Refresh.",
        "Makale yok — Yenile'ye bas.",
    ),
    (
        "Everything will be lost. This cannot be undone.",
        "Her şey kaybolacak. Bu geri alınamaz.",
    ),
    ("Save your password", "Şifreni kaydet"),
    ("{} note(s)", "{} not"),
    ("no vault", "kasa yok"),
    ("Backlinks ({})", "Geri bağlantılar ({})"),
    ("Unlinked mentions ({})", "Bağlantısız anmalar ({})"),
    ("Erase everything?", "Her şey silinsin mi?"),
    // ----------------------------------------------------------- status
    ("Note deleted", "Not silindi"),
    ("Note bookmarked", "Not yer imine eklendi"),
    ("Bookmark removed", "Yer imi kaldırıldı"),
    ("Random note", "Rastgele not"),
    ("Settings saved", "Ayarlar kaydedildi"),
    ("Vault is locked", "Kasa kilitli"),
    ("Vault locked", "Kasa kilitlendi"),
    ("Open a vault first", "Önce bir kasa aç"),
    ("Open a note first", "Önce bir not aç"),
    ("Table updated", "Tablo güncellendi"),
    ("No table found in this note", "Bu notta tablo bulunamadı"),
    ("Property saved", "Özellik kaydedildi"),
    ("Property removed", "Özellik silindi"),
    ("Second pane closed", "İkinci panel kapatıldı"),
    ("Loaded into second pane", "İkinci panele yüklendi"),
    (
        "Recording… run Voice note again to stop",
        "Kaydediliyor… durdurmak için Ses notu komutunu tekrar çalıştır",
    ),
    ("Could not start ffmpeg", "ffmpeg başlatılamadı"),
    ("Spell check on", "Yazım denetimi açık"),
    ("Spell check off", "Yazım denetimi kapalı"),
    ("Vim mode on", "Vim modu açık"),
    ("Vim mode off", "Vim modu kapalı"),
    (
        "Concealed markdown (WYSIWYG) on",
        "Gizlenen markdown (WYSIWYG) açık",
    ),
    ("Raw markdown on", "Ham markdown açık"),
    (
        "Linked pane on — links open beside",
        "Bağlantılı panel açık — bağlantılar yanda açılır",
    ),
    ("Linked pane off", "Bağlantılı panel kapalı"),
    // ------------------------------------------------------- status detail
    ("Unlock first", "Önce kilidi aç"),
    ("Vault opened", "Kasa açıldı"),
    ("Unlock failed", "Kilit açılamadı"),
    ("Vault locked", "Kasa kilitlendi"),
    ("Nothing to edit", "Düzenlenecek bir şey yok"),
    ("Invalid line range", "Geçersiz satır aralığı"),
    ("No notes to pick from", "Seçilecek not yok"),
    ("Pick a different note", "Farklı bir not seç"),
    ("Enter the note to merge into", "Birleştirilecek notu gir"),
    ("Could not read the target note", "Hedef not okunamadı"),
    ("Could not create the new note", "Yeni not oluşturulamadı"),
    ("Split note into a new file", "Not yeni bir dosyaya bölündü"),
    ("Enter a property name", "Bir özellik adı gir"),
    ("Pick a tag to rename", "Yeniden adlandırılacak etiketi seç"),
    ("Enter a new tag name", "Yeni etiket adını gir"),
    ("No matches to replace", "Değiştirilecek eşleşme yok"),
    ("Enter something to replace", "Değiştirilecek bir şey gir"),
    ("No templates in '{}/'", "'{}/' içinde şablon yok"),
    ("Save failed: {}", "Kaydetme başarısız: {}"),
    ("Saved · {}", "Kaydedildi · {}"),
    ("Created · {}", "Oluşturuldu · {}"),
    ("Created sub-note · {}", "Alt not oluşturuldu · {}"),
    ("Renamed to {}", "Yeniden adlandırıldı: {}"),
    ("Rename failed: {}", "Yeniden adlandırma başarısız: {}"),
    ("Delete failed: {}", "Silme başarısız: {}"),
    ("Property '{}' saved", "'{}' özelliği kaydedildi"),
    ("Property '{}' removed", "'{}' özelliği silindi"),
    (
        "Deleted tag '{}' from every note",
        "'{}' etiketi tüm notlardan silindi",
    ),
    (
        "Renamed tag '{}' → '{}'",
        "'{}' etiketi '{}' olarak değiştirildi",
    ),
    ("Edited {} line(s)", "{} satır düzenlendi"),
    ("Replaced {} occurrence(s)", "{} eşleşme değiştirildi"),
    ("No matches for '{}'", "'{}' için eşleşme yok"),
    ("Copied {}", "Kopyalandı: {}"),
    ("Saved voice note · {}", "Ses notu kaydedildi · {}"),
    (
        "Recording failed (is ffmpeg installed?)",
        "Kayıt başarısız (ffmpeg kurulu mu?)",
    ),
    ("Merged into the target note", "Hedef notla birleştirildi"),
    ("Table updated", "Tablo güncellendi"),
    ("Added PDF card", "PDF kartı eklendi"),
    ("Canvas saved", "Canvas kaydedildi"),
    ("Opened canvas", "Canvas açıldı"),
    ("Created canvas", "Canvas oluşturuldu"),
    ("Canvas save failed: {}", "Canvas kaydetme başarısız: {}"),
    ("Create canvas failed: {}", "Canvas oluşturma başarısız: {}"),
    ("Could not read canvas", "Canvas okunamadı"),
    (
        "Copy the username, password or website",
        "Kullanıcı adını, şifreyi veya siteyi kopyala",
    ),
];

/// Translate an English UI string for the current language.
///
/// Untranslated strings fall back to the English original.
pub fn t(src: &'static str) -> &'static str {
    match current() {
        Lang::En => src,
        Lang::Tr => TR
            .iter()
            .find(|(en, _)| *en == src)
            .map(|(_, tr)| *tr)
            .unwrap_or(src),
    }
}

/// Translate a template containing `{}` placeholders and fill them in.
pub fn tf(src: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = t(src).to_string();
    for a in args {
        if let Some(pos) = out.find("{}") {
            out.replace_range(pos..pos + 2, &a.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_identity() {
        set(Lang::En);
        assert_eq!(t("New note"), "New note");
    }

    #[test]
    fn turkish_translates_and_falls_back() {
        set(Lang::Tr);
        assert_eq!(t("New note"), "Yeni not");
        assert_eq!(t("Some untranslated string"), "Some untranslated string");
        assert_eq!(tf("Bookmarks ({})", &[&3]), "Yer imleri (3)");
        set(Lang::En);
    }

    #[test]
    fn codes_round_trip() {
        assert_eq!(Lang::from_code("tr"), Lang::Tr);
        assert_eq!(Lang::from_code("TR-tr"), Lang::Tr);
        assert_eq!(Lang::from_code("de"), Lang::En);
    }
}
