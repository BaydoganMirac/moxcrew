use serde_json::Value;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Tr,
    En,
}

impl Language {
    pub fn from_code(code: &str) -> Self {
        if code == "en" {
            Self::En
        } else {
            Self::Tr
        }
    }
    pub fn from_snapshot(snapshot: &Value) -> Self {
        Self::from_code(
            snapshot["desktopPreferences"]["language"]
                .as_str()
                .unwrap_or("tr"),
        )
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Tr => "tr",
            Self::En => "en",
        }
    }
}

pub fn tr_for_snapshot<'a>(snapshot: &Value, text: &'a str) -> Cow<'a, str> {
    tr(Language::from_snapshot(snapshot), text)
}

pub fn tr(language: Language, text: &str) -> Cow<'_, str> {
    if text
        .bytes()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_' || b == b' ')
    {
        if let Some((en, translated)) = reason(&text.replace(' ', "_")) {
            return Cow::Borrowed(if language == Language::En {
                en
            } else {
                translated
            });
        }
    }
    if language == Language::En {
        return Cow::Borrowed(text);
    }
    let translated = match text {
        "In Finder, open the folder's Get Info > Sharing & Permissions and allow your account to Read & Write. If the drive is read-only, choose a folder on a writable drive." => "Finder’da klasörün Bilgi Ver > Paylaşım ve İzinler bölümünde hesabınıza Okuma ve Yazma izni verin. Disk salt okunursa yazılabilir bir diskte klasör seçin.",
        "This folder is linked to {0}. Choose another folder or unlink it there first." => "Bu klasör {0} projesine bağlı. Başka klasör seçin veya önce o projeden bağlantısını kaldırın.",
        "Your projects, local folders and shared memory." => "Projeleriniz, yerel klasörleriniz ve ortak hafızanız.",
        "Local data could not be opened safely. Close Patchfleet and back up its data before checking the storage or access permissions." => "Yerel veriler güvenle açılamadı. Depolama veya erişim izinlerini kontrol etmeden önce Patchfleet’i kapatıp verilerini yedekleyin.",
        "Keep your folders, agents and project memory together." => "Klasörlerinizi, ajanlarınızı ve proje hafızasını bir arada tutun.",
        "New project" => "Yeni proje", "{0} projects · {1} local folders" => "{0} proje · {1} yerel klasör",
        "{0} folders · {1} sources" => "{0} klasör · {1} kaynak", "Connect a local folder" => "Bir proje klasörü bağlayın",
        "Choose the Git repository where your agents will work." => "Ajanlarınızın çalışacağı Git deposunu seçin.",
        "Choose folder" => "Klasör seç", "Change folder" => "Klasörü değiştir", "Add another folder" => "Başka klasör ekle",
        "Removed folder" => "Kaldırılmış klasör", "Link an existing folder" => "Kayıtlı bir klasör bağla", "Link folder" => "Projeye bağla",
        "Project details" => "Proje ayrıntıları", "LOCAL FOLDERS" => "YEREL KLASÖRLER", "Add folder" => "Klasör ekle", "Remove folder" => "Klasörü kaldır",
        "No folders added yet. Choose a repository using the system folder picker." => "Henüz klasör eklenmedi. Sistem klasör seçicisiyle bir Git deposu seçin.",
        "Add a local folder" => "Yerel klasör ekle", "Your agents work here. Files stay on this device." => "Ajanlarınız burada çalışır. Dosyalar bu cihazda kalır.",
        "Git folder found. Access will be checked again when you add it." => "Git deposu bulundu. Klasör eklenirken erişim yeniden kontrol edilir.",
        "This folder is already added. You can link it from the project card." => "Bu klasör zaten kayıtlı. Proje kartından bağlayabilirsiniz.",
        "Folder label · optional" => "Klasör adı · isteğe bağlı", "After adding this folder, link it from the project card." => "Klasörü ekledikten sonra proje kartından bağlayın.",
        "The folder label is limited to 160 characters." => "Klasör adı en fazla 160 karakter olabilir.", "Cancel" => "Vazgeç", "No folder selected" => "Klasör seçilmedi",
        "Choose your Git repository folder" => "Git projenizin klasörünü seçin", "Choose another folder" => "Başka klasör seç",
        "Use a registered folder instead" => "Kayıtlı klasör kullan", "Select a folder for this task, or choose a registered folder above." => "Bu iş için bir klasör seçin veya yukarıdan kayıtlı bir klasör kullanın.",
        "Choose an accessible Git folder before adding this task." => "İşi eklemeden önce erişilebilir bir Git klasörü seçin.",
        "Folder access needs attention" => "Klasör erişimini kontrol edin", "Select the repository folder" => "Git deposunun klasörünü seçin",
        "Action completed" => "İşlem tamamlandı", "Action was not applied" => "İşlem uygulanmadı", "Action could not be completed" => "İşlem tamamlanamadı",
        "Dismiss" => "Kapat", "Technical details" => "Teknik ayrıntılar", "Open permission settings" => "İzin ayarlarını aç",
        "The local action was recorded." => "Yerel işlem kaydedildi.",
        "Review the current state and try the action again. Details are available below." => "Güncel durumu kontrol edip işlemi yeniden deneyin. Ayrıntılar aşağıda.",
        "Select the folder again and allow access if macOS asks. If access was denied, enable Patchfleet in System Settings > Privacy & Security > Files and Folders." => "Klasörü yeniden seçin; macOS sorarsa erişime izin verin. Erişim reddedildiyse Sistem Ayarları > Gizlilik ve Güvenlik > Dosyalar ve Klasörler bölümünden Patchfleet’i etkinleştirin.",
        "Select the folder again. In the folder's Properties > Security, allow your account to read and modify it. If Windows blocked an agent, review that agent's access separately." => "Klasörü yeniden seçin. Klasörün Özellikler > Güvenlik bölümünde hesabınıza okuma ve değiştirme izni verin. Windows bir ajanı engellediyse o ajanın erişimini ayrıca kontrol edin.",
        "Select the folder again. Check your account's read, write and traversal permissions in the file manager. A read-only drive must be mounted writable." => "Klasörü yeniden seçin. Dosya yöneticisinde hesabınızın okuma, yazma ve klasöre giriş izinlerini kontrol edin. Salt okunur disk yazılabilir olarak bağlanmalıdır.",
        "Projects" => "Projeler", "Memory" => "Hafıza", "Work" => "İşler", "Teams" => "Takımlar",
        "Agents" => "Ajanlar", "Providers" => "Sağlayıcılar", "Cloud" => "Bulut",
        "Stable projects, linked worktrees and shared local context." => "Kalıcı projeler, bağlı çalışma ağaçları ve ortak yerel bağlam.",
        "Project sources with provenance. ADR extraction comes next." => "Kaynağı belli proje kayıtları. ADR çıkarımı sonraki adımda.",
        "Queue a bounded task, start Codex, and verify its durable outcome." => "Sınırları belli bir işi sıraya alın, Codex’i başlatın ve sonucunu izleyin.",
        "Choose agents and a bounded plan. Owner decisions stay explicit." => "Ajanları ve sınırları belli bir planı seçin. Kararlar sizin onayınızla geçerli olur.",
        "Inspect instructions, permissions and limits before selecting an agent." => "Ajan seçmeden önce talimatlarını, izinlerini ve sınırlarını inceleyin.",
        "Capabilities reflect the installed CLIs and last local observation." => "Yetenekler, kurulu CLI’ları ve son yerel gözlemi yansıtır.",
        "An optional sanitized remote view. Local operation stays available." => "İsteğe bağlı, özel verilerden arındırılmış uzak görünüm. Yerel çalışma devam eder.",
        "Add a task name (up to 160 characters)." => "En fazla 160 karakterlik bir iş adı girin.",
        "Add a bounded instruction (up to 50,000 characters)." => "En fazla 50.000 karakterlik, sınırları belli bir talimat girin.",
        "Choose a registered worktree or one absolute worktree path." => "Kayıtlı bir çalışma ağacı seçin ya da tam bir çalışma ağacı yolu girin.",
        "That worktree is no longer registered. Choose a current worktree." => "Bu çalışma ağacı artık kayıtlı değil. Güncel bir çalışma ağacı seçin.",
        "Add a team name (up to 80 characters)." => "En fazla 80 karakterlik bir takım adı girin.",
        "Describe the team goal (up to 10,000 characters)." => "Takımın hedefini en fazla 10.000 karakterle açıklayın.",
        "Choose a registered worktree." => "Kayıtlı bir çalışma ağacı seçin.",
        "Choose an available team template." => "Kullanılabilir bir takım şablonu seçin.",
        "Choose an installed orchestrator." => "Kurulu bir koordinatör seçin.",
        "Choose at least one worker agent." => "En az bir çalışan ajan seçin.",
        "Select each worker pack only once." => "Her çalışan ajan paketini yalnızca bir kez seçin.",
        "A selected worker is no longer installed." => "Seçilen çalışan ajan artık kurulu değil.",
        "Use concurrency 1–4, retries 0–3 and a time budget of 5–480 minutes." => "Eşzamanlı iş sayısı 1–4, yeniden deneme 0–3 ve süre 5–480 dakika olmalı.",
        "Choose a current project before writing a message." => "Mesaj yazmadan önce güncel bir proje seçin.",
        "Add a message title of up to 160 characters." => "En fazla 160 karakterlik bir mesaj başlığı girin.",
        "Add source text of up to 50,000 characters." => "En fazla 50.000 karakterlik kaynak metni girin.",
        "WORKSPACE" => "ÇALIŞMA ALANI", "Language" => "Dil", "Local engine" => "Yerel motor",
        "Refresh providers" => "Sağlayıcıları yenile", "Applying action…" => "İşlem uygulanıyor…",
        "YOUR LOCAL WORKSPACE" => "YEREL ÇALIŞMA ALANINIZ", "Create your first project" => "İlk projenizi oluşturun",
        "Give your project a stable home for worktrees, tasks and local context." => "Çalışma ağaçlarını, işleri ve yerel bağlamı kalıcı bir projede birleştirin.",
        "Create a project" => "Proje oluştur", "{0} sources" => "{0} kaynak", "Open memory" => "Hafızayı aç",
        "Linked worktrees share this project's source context." => "Bağlı çalışma ağaçları bu projenin kaynak bağlamını paylaşır.",
        "Removed worktree" => "Kaldırılmış çalışma ağacı", "Unlink" => "Bağlantıyı kaldır",
        "Register an unlinked worktree below to link it here." => "Buraya bağlamak için aşağıdan bağımsız bir çalışma ağacı kaydedin.",
        "Link a registered worktree" => "Kayıtlı çalışma ağacı bağla", "Link worktree" => "Çalışma ağacını bağla",
        "Project identity" => "Proje kimliği", "Revision {0} · created {1}" => "Revizyon {0} · oluşturulma {1}",
        "REGISTERED WORKTREES" => "KAYITLI ÇALIŞMA AĞAÇLARI",
        "Worktrees are execution folders. Unlinked worktrees remain usable for tasks and teams." => "Çalışma ağaçları işlerin yürütüldüğü klasörlerdir. Bağımsız olanlar da iş ve takımlarda kullanılabilir.",
        "Unlinked" => "Bağımsız", "Unregister" => "Kaydı kaldır", "Register a worktree" => "Çalışma ağacı kaydet",
        "Project" => "Proje", "A stable identity independent of repository paths." => "Depo yollarından bağımsız, kalıcı bir kimlik.",
        "Project name" => "Proje adı", "e.g. Patchfleet, Website, Mobile app" => "örn. Patchfleet, Web sitesi, Mobil uygulama",
        "Create project" => "Proje oluştur", "Enter a project name of up to 80 characters." => "En fazla 80 karakterlik bir proje adı girin.",
        "Link worktrees after creating the project. Existing folders are not grouped automatically." => "Projeyi oluşturduktan sonra çalışma ağaçlarını bağlayın. Mevcut klasörler otomatik gruplanmaz.",
        "Register worktree" => "Çalışma ağacı kaydet", "An existing Git worktree used to execute tasks." => "İşlerin yürütüleceği mevcut bir Git çalışma ağacı.",
        "Worktree path" => "Çalışma ağacı yolu", "Worktree label · optional" => "Çalışma ağacı adı · isteğe bağlı",
        "e.g. Website, API, Mobile app" => "örn. Web sitesi, API, Mobil uygulama",
        "Enter the absolute path of an existing Git worktree." => "Mevcut bir Git çalışma ağacının tam yolunu girin.",
        "The worktree path is too long." => "Çalışma ağacı yolu çok uzun.", "The worktree label is limited to 160 characters." => "Çalışma ağacı adı en fazla 160 karakter olabilir.",
        "Files stay where they are. Nothing is uploaded." => "Dosyalar yerinde kalır. Hiçbir şey yüklenmez.",
        "Create a project first" => "Önce bir proje oluşturun",
        "Project memory keeps owner sources together across explicitly linked worktrees." => "Proje hafızası, açıkça bağlanan çalışma ağaçlarının kullanıcı kaynaklarını bir arada tutar.",
        "Source context" => "Kaynak bağlamı", "ADR extraction comes next. These records are not accepted decisions." => "ADR çıkarımı sonraki adımda. Bu kayıtlar onaylanmış karar değildir.",
        "Choose a project to read or write its local source context." => "Yerel kaynak bağlamını okumak veya yazmak için proje seçin.",
        "Keyword filter" => "Anahtar kelime filtresi", "Words in source titles or text" => "Kaynak başlığında veya metninde ara",
        "Apply keyword" => "Filtreyi uygula", "Keyword filter is limited to 160 characters." => "Anahtar kelime filtresi en fazla 160 karakter olabilir.",
        "Refresh memory" => "Hafızayı yenile", "Edit project message" => "Proje mesajını düzenle",
        "Captured task and team sources remain immutable." => "Kaydedilmiş iş ve takım kaynakları değiştirilemez.",
        "Title" => "Başlık", "Source text" => "Kaynak metni", "Save message" => "Mesajı kaydet", "Cancel edit" => "Düzenlemeyi iptal et",
        "This source is no longer current on this page. Cancel and reopen it before saving." => "Bu kaynak artık bu sayfada güncel değil. Kaydetmeden önce iptal edip yeniden açın.",
        "Write a project message" => "Proje mesajı yaz", "Message title" => "Mesaj başlığı", "Add message" => "Mesaj ekle",
        "Owner-authored context stays local. Adding it does not accept an ADR." => "Yazdığınız bağlam yerelde kalır. Eklemek, bir ADR’yi onaylamak değildir.",
        "{0} sources matching keyword" => "Anahtar kelimeyle eşleşen {0} kaynak",
        "Previous sources" => "Önceki kaynaklar", "Page {0}" => "Sayfa {0}", "Next sources" => "Sonraki kaynaklar",
        "No sources yet" => "Henüz kaynak yok", "No keyword matches" => "Anahtar kelimeyle eşleşme yok",
        "Write a project message or submit work through a linked worktree." => "Proje mesajı yazın veya bağlı bir çalışma ağacında iş başlatın.",
        "Revision {0}" => "Revizyon {0}", "Full source and provenance" => "Tam kaynak ve köken bilgisi",
        "Author {0} · updated {1}" => "Yazan {0} · güncelleme {1}", "Origin {0} · {1} · {2}" => "Köken {0} · {1} · {2}",
        "Edit message" => "Mesajı düzenle", "Captured source · read only" => "Kaydedilmiş kaynak · salt okunur",
        "The current source page is unavailable. Use Refresh memory to retry a failed read." => "Güncel kaynak sayfası kullanılamıyor. Yeniden denemek için hafızayı yenileyin.",
        "Codex ready" => "Codex hazır", "Codex not ready" => "Codex hazır değil",
        "Runs stay local. Every action leaves a receipt." => "Çalıştırmalar yerelde kalır. Her işlem bir sonuç kaydı bırakır.",
        "TASK QUEUE" => "İŞ KUYRUĞU", "Nothing in the queue" => "Kuyruk boş",
        "Create a task and it will appear here before an agent starts." => "Bir iş oluşturun; ajan başlamadan önce burada görünür.",
        "Previous" => "Önceki", "Page {0} of {1} · {2} tasks" => "Sayfa {0}/{1} · {2} iş", "Next" => "Sonraki",
        "Unregistered worktree" => "Kayıtsız çalışma ağacı", "Local task details" => "Yerel iş ayrıntıları", "Run {0} · turn {1}" => "Çalıştırma {0} · tur {1}",
        "Start Codex" => "Codex’i başlat", "Remove" => "Kaldır", "Queue retry" => "Yeniden denemeyi sıraya al", "Cancel run" => "Çalıştırmayı iptal et",
        "Control belongs to an earlier host session. Refresh to reconcile safely." => "Kontrol önceki bir oturuma ait. Durumu güvenle uzlaştırmak için yenileyin.",
        "Recent durable activity" => "Son kalıcı işlemler", "No receipts yet. Accepted actions leave a durable outcome here." => "Henüz sonuç kaydı yok. Kabul edilen işlemlerin kalıcı sonuçları burada görünür.",
        "New task" => "Yeni iş", "One clear outcome for Codex." => "Codex için tek ve net bir hedef.", "Task name" => "İş adı",
        "Use another Git worktree once" => "Başka bir Git çalışma ağacını bir kez kullan", "Absolute worktree path" => "Tam çalışma ağacı yolu",
        "Leave this blank when selecting a registered project." => "Kayıtlı bir çalışma ağacı seçtiyseniz burayı boş bırakın.",
        "Instruction" => "Talimat", "Add to queue" => "Kuyruğa ekle", "Review the queued task before starting. Instructions stay local." => "Başlatmadan önce kuyruktaki işi inceleyin. Talimatlar yerelde kalır.",
        "Form a local team" => "Yerel takım kur", "Team name" => "Takım adı", "Template" => "Şablon", "Orchestrator" => "Koordinatör",
        "Agent {0}{1}" => "Ajan {0}{1}", " (required)" => " (zorunlu)", "Concurrency" => "Eşzamanlı iş", "Retries" => "Yeniden deneme", "Minutes" => "Dakika",
        "Continue independent tasks after a failure" => "Bir hata sonrasında bağımsız işlere devam et",
        "Goal and acceptance criteria" => "Hedef ve kabul ölçütleri", "Create team plan" => "Takım planı oluştur",
        "The team is saved as a draft. Review it before starting." => "Takım taslak olarak kaydedilir. Başlatmadan önce inceleyin.",
        "No team yet" => "Henüz takım yok", "Select an orchestrator, worker roles and a bounded template above." => "Yukarıdan koordinatör, çalışan roller ve sınırları belli bir şablon seçin.",
        "{0} · concurrency {1} · {2} retries" => "{0} · {1} eşzamanlı iş · {2} yeniden deneme",
        "Start team" => "Takımı başlat", "Run ready work" => "Hazır işleri çalıştır", "Cancel team" => "Takımı iptal et",
        "{0} · attempt {1}/{2}" => "{0} · deneme {1}/{2}", "Unassigned" => "Atanmamış", "Record answer" => "Yanıtı kaydet",
        "Decision note (optional)" => "Karar notu (isteğe bağlı)", "Approve" => "Onayla", "Reject" => "Reddet",
        "Agents and controls" => "Ajanlar ve kontroller", "Cancel agent" => "Ajanı iptal et",
        "{0} inspectable local packs" => "İncelenebilir {0} yerel paket", "Inspect pack contract" => "Paket sözleşmesini incele",
        "Provider: {0} · {1} minutes · {2} attempts" => "Sağlayıcı: {0} · {1} dakika · {2} deneme",
        "Permissions: {0}" => "İzinler: {0}", "Quality checks: {0}" => "Kalite kontrolleri: {0}", "Expected output" => "Beklenen çıktı",
        "Remove custom pack" => "Özel paketi kaldır", "Install a custom pack" => "Özel paket kur", "Inspectable JSON manifest" => "İncelenebilir JSON bildirimi",
        "Install custom pack" => "Özel paketi kur", "Packs are local data. They cannot load executable plugins or widen the Codex sandbox." => "Paketler yerel veridir. Çalıştırılabilir eklenti yükleyemez veya Codex’in izin alanını genişletemez.",
        "Providers have not been observed" => "Sağlayıcılar henüz gözlemlenmedi", "Refresh providers to check the installed CLIs and proven controls." => "Kurulu CLI’ları ve doğrulanmış kontrolleri görmek için sağlayıcıları yenileyin.",
        "CLI version: {0}" => "CLI sürümü: {0}", "Not available" => "Kullanılamıyor", "supported" => "destekleniyor", "unavailable" => "kullanılamıyor",
        "Recent observation: {0} · explicit live status: {1}" => "Son gözlem: {0} · açık canlı durum: {1}",
        "Stored snapshot from {0}" => "Kayıtlı durum: {0}", "Recent sessions ({0})" => "Son oturumlar ({0})",
        "No recent interactive sessions returned." => "Yakın tarihli etkileşimli oturum bulunamadı.", "Last observed {0}" => "Son gözlem {0}",
        "Paired" => "Eşleştirilmiş", "Local only" => "Yalnızca yerel",
        "Only allowlisted operational status leaves this machine. Instructions, paths and transcripts stay local." => "Bu cihazdan yalnızca izin verilen çalışma durumu çıkar. Talimatlar, yollar ve konuşmalar yerelde kalır.",
        "Cloud settings need local recovery. Local work remains available." => "Bulut ayarlarının yerelde kurtarılması gerekiyor. Yerel işler kullanılabilir.",
        "Close Patchfleet and back up the private cloud.json file in its data directory. Explicitly repair or remove that file, then reopen the app." => "Patchfleet’i kapatın ve veri dizinindeki özel cloud.json dosyasını yedekleyin. Dosyayı açıkça onarın veya kaldırın, ardından uygulamayı yeniden açın.",
        "Pairing is unavailable until the local Cloud state can be read safely." => "Yerel bulut durumu güvenle okunana kadar eşleştirme kullanılamaz.",
        "Host {0}" => "Ana makine {0}", "Last successful sync: {0}" => "Son başarılı eşitleme: {0}",
        "Sync now" => "Şimdi eşitle", "Disconnect Cloud" => "Bulut bağlantısını kes", "Cloud URL" => "Bulut adresi", "Host name" => "Ana makine adı",
        "Pairing code" => "Eşleştirme kodu", "Pair this host" => "Bu ana makineyi eşleştir", "Enter a Cloud URL, host name and pairing code." => "Bulut adresi, ana makine adı ve eşleştirme kodu girin.",
        "Pack manifest exceeds 64 KiB." => "Paket bildirimi 64 KiB sınırını aşıyor.", "Pack manifest must be valid JSON." => "Paket bildirimi geçerli JSON olmalı.", "Pack manifest must be a JSON object." => "Paket bildirimi bir JSON nesnesi olmalı.",
        "Not recorded" => "Kaydedilmemiş", "Host access was revoked. Disconnect and pair again." => "Ana makinenin erişimi kaldırıldı. Bağlantıyı kesip yeniden eşleştirin.",
        "Cloud returned an incompatible response." => "Bulut uyumsuz bir yanıt döndürdü.", "Cloud rejected a conflicting snapshot. Local work stays available." => "Bulut çakışan bir durumu reddetti. Yerel işler kullanılabilir.",
        "Cloud sync is unavailable. Local work continues while the host retries." => "Bulut eşitlemesi kullanılamıyor. Ana makine yeniden denerken yerel çalışma devam eder.",
        "Registered worktree" => "Kayıtlı çalışma klasörü", "Choose…" => "Seçin…", "Unavailable selection" => "Seçim kullanılamıyor", "None" => "Yok",
        "Project message" => "Proje mesajı", "Task instruction" => "İş talimatı", "Team goal" => "Takım hedefi", "Owner answer" => "Kullanıcı yanıtı", "Approval note" => "Onay notu",
        "Stopping owned agents…" => "Uygulamanın ajanları durduruluyor…", "Retry the same action" => "Aynı işlemi yeniden dene",
        "The local host needs attention" => "Yerel ana makineyi kontrol edin",
        "Close another Patchfleet instance if it owns this data directory. Your local state is preserved." => "Bu veri dizinini kullanan başka bir Patchfleet varsa kapatın. Yerel verileriniz korunur.",
        "Retry opening the host" => "Ana makineyi yeniden açmayı dene", "Opening Patchfleet" => "Patchfleet açılıyor", "Loading the local Rust host…" => "Yerel ana makine yükleniyor…",
        "The local host is not ready for another action." => "Yerel ana makine yeni bir işlem için hazır değil.",
        "Language preference could not be read. Select a language to replace it." => "Dil tercihi okunamadı. Yeniden kaydetmek için bir dil seçin.",
        "available" => "hazır", "completed" => "tamamlandı", "applied" => "uygulandı", "failed" => "başarısız", "rejected" => "reddedildi", "timed_out" => "süre doldu",
        "queued" => "kuyrukta", "running" => "çalışıyor", "launching" => "başlatılıyor", "cancelling" => "iptal ediliyor", "cancelled" => "iptal edildi", "interrupted" => "kesildi",
        "blocked" => "engellendi", "draft" => "taslak", "active" => "aktif", "waiting" => "bekliyor", "waiting_question" => "yanıt bekliyor", "waiting_approval" => "onay bekliyor",
        "ready" => "hazır", "planned" => "planlandı", "pending" => "bekliyor", "idle" => "boşta", "unknown" => "bilinmiyor", "degraded" => "kısıtlı", "approved" => "onaylandı",
        "orchestrator" => "koordinatör", "developer" => "geliştirici", "designer" => "tasarımcı", "qa" => "kalite", "reviewer" => "inceleyen", "release" => "sürüm", "built-in" => "yerleşik", "local" => "yerel",
        "product" => "ürün", "design" => "tasarım", "frontend" => "arayüz", "backend" => "arka uç", "fullstack" => "uçtan uca", "review" => "inceleme", "security" => "güvenlik", "docs" => "dokümantasyon", "research" => "araştırma",
        "Product Manager" => "Ürün yöneticisi", "UI/UX Designer" => "Arayüz ve deneyim tasarımcısı", "Frontend Engineer" => "Arayüz geliştiricisi", "Backend Engineer" => "Arka uç geliştiricisi",
        "Full-stack Engineer" => "Uçtan uca geliştirici", "QA Engineer" => "Kalite mühendisi", "Code Reviewer" => "Kod inceleyicisi", "Security Reviewer" => "Güvenlik inceleyicisi",
        "Release Engineer" => "Sürüm mühendisi", "Documentation Writer" => "Dokümantasyon yazarı", "Researcher" => "Araştırmacı",
        "Plans a bounded task graph, assigns ready work, enforces gates, and reports one verified outcome." => "Sınırları belli bir iş planı kurar, hazır işleri atar, onay kapılarını uygular ve doğrulanmış sonuç bildirir.",
        "Turns an owner goal into explicit scope, acceptance criteria, risks, and delivery order." => "Kullanıcının hedefini açık kapsam, kabul ölçütleri, riskler ve teslim sırasına dönüştürür.",
        "Defines accessible interaction, responsive states, content hierarchy, and implementation-ready design decisions." => "Erişilebilir etkileşimleri, farklı ekran durumlarını, içerik hiyerarşisini ve uygulanabilir tasarım kararlarını tanımlar.",
        "Implements accessible responsive product surfaces that follow repository patterns." => "Depo kurallarına uygun, erişilebilir ve farklı ekranlara uyumlu arayüzler geliştirir.",
        "Implements validated domain, API, persistence, authorization, and failure behavior." => "Doğrulanan iş kuralları, API, kalıcı depolama, yetkilendirme ve hata davranışlarını geliştirir.",
        "Delivers one complete vertical slice across interface, server, storage, and tests." => "Arayüz, sunucu, depolama ve testleri kapsayan tamamlanmış bir iş parçası teslim eder.",
        "Finds boundary failures and adds the smallest deterministic checks that reproduce them." => "Sınır durumlarındaki hataları bulur ve yeniden üreten küçük, tutarlı kontroller ekler.",
        "Reviews correctness, security, regressions, and missing tests without rewriting unrelated code." => "İlgisiz kodu yeniden yazmadan doğruluğu, güvenliği, gerilemeleri ve eksik testleri inceler.",
        "Threat-models trust boundaries and reports exploitable authorization, injection, secret, and data-loss risks." => "Güven sınırlarını analiz eder; istismar edilebilir yetki, enjeksiyon, gizli veri ve veri kaybı risklerini raporlar.",
        "Verifies builds, migrations, rollback, configuration, packaging, and release evidence without publishing externally." => "Dışarıya yayınlamadan derlemeleri, geçişleri, geri alma planını, yapılandırmayı, paketlemeyi ve sürüm kanıtını doğrular.",
        "Writes accurate user and operator guidance from the implemented behavior and current product contract." => "Uygulanan davranış ve güncel ürün sözleşmesine göre doğru kullanıcı ve işletim kılavuzları yazar.",
        "Collects decision-relevant evidence, distinguishes facts from inference, and records concise recommendations." => "Karar için gerekli kanıtları toplar, gerçeklerle çıkarımları ayırır ve kısa öneriler kaydeder.",
        " Stay inside the assigned task, repository rules, declared permissions, and acceptance criteria. Do not perform external publication, deployment, billing, or messaging. Return a concise outcome, changed files, checks run, blockers, and handoff evidence." => " Atanan iş, depo kuralları, belirtilen izinler ve kabul ölçütleri içinde kalın. Dışarıya yayınlama, dağıtım, faturalama veya mesajlaşma yapmayın. Kısa sonuç, değişen dosyalar, çalıştırılan kontroller, engeller ve teslim kanıtı sunun.",
        "Outcome, changed files, checks run, unresolved blockers, and a concise handoff for the orchestrator." => "Sonuç, değişen dosyalar, çalıştırılan kontroller, çözülmemiş engeller ve koordinatör için kısa bir teslim özeti.",
        "Product feature" | "product feature" => "Ürün özelliği", "Bug fix" | "bug fix" => "Hata düzeltme", "SaaS launch" | "saas launch" => "SaaS lansmanı",
        "Design + frontend" | "design frontend" => "Tasarım ve arayüz", "Security audit" | "security audit" => "Güvenlik incelemesi", "Release" => "Sürüm",
        "Plan, design, implement, test, and review a product slice." => "Bir ürün parçasını planlayın, tasarlayın, uygulayın, test edin ve inceleyin.",
        "Diagnose, fix, verify, and review one reproducible defect." => "Tekrar üretilebilen bir hatayı teşhis edin, düzeltin, doğrulayın ve inceleyin.",
        "Check product, security, quality, and release readiness behind an owner gate." => "Kullanıcı onayıyla ürünün, güvenliğin, kalitenin ve sürümün hazır olduğunu kontrol edin.",
        "Define an accessible interaction and implement its responsive frontend." => "Erişilebilir bir etkileşim tanımlayın ve farklı ekranlara uyumlu arayüzünü geliştirin.",
        "Threat-model, remediate selected risks, verify, and review." => "Tehditleri modelleyin, seçilen riskleri giderin, doğrulayın ve inceleyin.",
        "Verify and prepare one rollback-safe release behind an owner gate." => "Kullanıcı onayıyla güvenle geri alınabilir bir sürümü doğrulayın ve hazırlayın.",
        "Confirm the release target and any constraints the release agent must follow." => "Sürüm hedefini ve sürüm ajanının uyması gereken sınırları onaylayın.",
        "Codex CLI is not installed or is not on PATH." => "Codex CLI kurulu değil veya PATH üzerinde bulunamıyor.",
        "Codex CLI could not be checked." => "Codex CLI kontrol edilemedi.", "Codex CLI version check timed out." => "Codex CLI sürüm kontrolünün süresi doldu.",
        "Codex CLI returned an unsupported version." => "Codex CLI desteklenmeyen bir sürüm döndürdü.", "Codex reported an observation error." => "Codex bir gözlem hatası bildirdi.",
        "Codex observation stopped unexpectedly." => "Codex gözlemi beklenmedik biçimde durdu.", "Codex observation could not start." => "Codex gözlemi başlatılamadı.",
        "Codex observation timed out." => "Codex gözleminin süresi doldu.", "Codex rejected the observation request." => "Codex gözlem isteğini reddetti.", "Codex returned an unsupported response." => "Codex desteklenmeyen bir yanıt döndürdü.",
        "Claude Code CLI is not installed or is not on PATH." => "Claude Code CLI kurulu değil veya PATH üzerinde bulunamıyor.",
        "Claude Code CLI could not be checked." => "Claude Code CLI kontrol edilemedi.", "Claude Code CLI version check timed out." => "Claude Code CLI sürüm kontrolünün süresi doldu.",
        "Claude Code CLI returned an unsupported version." => "Claude Code CLI desteklenmeyen bir sürüm döndürdü.", "This Claude Code version lacks the supported Agent View surface." => "Bu Claude Code sürümünde desteklenen Agent View arayüzü yok.",
        "Claude Code Agent View could not be checked." => "Claude Code Agent View kontrol edilemedi.", "Claude Code Agent View timed out." => "Claude Code Agent View kontrolünün süresi doldu.",
        "Claude Code Agent View returned an unsupported response." => "Claude Code Agent View desteklenmeyen bir yanıt döndürdü.", "Claude Code Agent View returned an unsupported session shape." => "Claude Code Agent View desteklenmeyen bir oturum biçimi döndürdü.",
        "Gemini CLI is not installed or is not on PATH." => "Gemini CLI kurulu değil veya PATH üzerinde bulunamıyor.", "Gemini CLI version check timed out." => "Gemini CLI sürüm kontrolünün süresi doldu.",
        "Gemini CLI returned an unsupported version." => "Gemini CLI desteklenmeyen bir sürüm döndürdü.", "Gemini CLI hook setup is required." => "Gemini CLI hook kurulumu gerekli.", "Gemini CLI could not be checked." => "Gemini CLI kontrol edilemedi.",
        _ => return Cow::Borrowed(text),
    };
    Cow::Borrowed(translated)
}

pub fn catalog(language: Language, raw: &str) -> Cow<'_, str> {
    const SUFFIX:&str=" Stay inside the assigned task, repository rules, declared permissions, and acceptance criteria. Do not perform external publication, deployment, billing, or messaging. Return a concise outcome, changed files, checks run, blockers, and handoff evidence.";
    if language == Language::Tr {
        if let Some(description) = raw.strip_suffix(SUFFIX) {
            let translated = tr(language, description);
            if translated != description {
                return Cow::Owned(format!("{}{}", translated, tr(language, SUFFIX)));
            }
        }
    }
    tr(language, raw)
}

pub fn format(language: Language, key: &str, args: &[&str]) -> String {
    let template = tr(language, key);
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template.as_ref();
    while let Some(start) = remaining.find('{') {
        rendered.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        if let Some(end) = remaining.find('}') {
            if let Some(value) = remaining[1..end]
                .parse::<usize>()
                .ok()
                .and_then(|i| args.get(i))
            {
                rendered.push_str(value);
                remaining = &remaining[end + 1..];
                continue;
            }
        }
        rendered.push('{');
        remaining = &remaining[1..];
    }
    rendered.push_str(remaining);
    rendered
}

fn reason(code: &str) -> Option<(&'static str, &'static str)> {
    Some(match code {
        "WORKSPACE_PATH_INVALID" => ("The folder path cannot be used. Choose it with the folder picker.", "Klasör yolu kullanılamıyor. Klasör seçiciden yeniden seçin."),
        "WORKSPACE_FOLDER_NOT_FOUND" => ("This folder no longer exists. Reconnect its drive or choose another folder.", "Bu klasör bulunamıyor. Diskini yeniden bağlayın veya başka klasör seçin."),
        "WORKSPACE_NOT_DIRECTORY" => ("Select a folder, rather than a file.", "Bir dosya yerine klasör seçin."),
        "WORKSPACE_UNSAFE_ROOT" => ("Choose a specific project folder. Your home or drive root cannot be used as an agent workspace.", "Belirli bir proje klasörü seçin. Kullanıcı klasörü veya diskin kökü ajan çalışma alanı olamaz."),
        "WORKSPACE_NOT_GIT_ROOT" => ("This folder is not a Git repository. Select the project folder containing .git, rather than its parent folder.", "Bu klasör bir Git deposu değil. Üst klasör yerine içinde .git bulunan asıl proje klasörünü seçin."),
        "WORKSPACE_ACCESS_DENIED" => ("Patchfleet cannot read or enter this folder. Select it again to request access, then check the folder permissions if needed.", "Patchfleet bu klasörü okuyamıyor veya açamıyor. Erişim istemek için yeniden seçin; gerekirse klasör izinlerini kontrol edin."),
        "WORKSPACE_READ_ONLY" => ("This folder can be read but cannot be changed. Give your account write access or choose a writable folder.", "Bu klasör okunabiliyor ancak değiştirilemiyor. Hesabınıza yazma izni verin veya yazılabilir bir klasör seçin."),
        "WORKSPACE_UNAVAILABLE" => ("The folder could not be checked. Reconnect its drive and select it again.", "Klasör kontrol edilemedi. Disk bağlantısını kontrol edip yeniden seçin."),
        "WORKSPACE_NOT_ALLOWED" => ("The agent's folder is unavailable or no longer meets the workspace requirements. Restore its Git folder and check access before starting again.", "Ajanın klasörü kullanılamıyor veya çalışma alanı koşullarını artık sağlamıyor. Yeniden başlatmadan önce Git klasörünü ve erişim izinlerini kontrol edin."),
        "HOST_BUSY" => ("The local host is still busy. Wait for the current action, then try again.", "Yerel ana makine hâlâ meşgul. Güncel işlemin bitmesini bekleyip yeniden deneyin."),
        "ACTION_RECORDED" => ("Action recorded.", "İşlem kaydedildi."),
        "OBSERVED" => ("Providers refreshed.", "Sağlayıcılar yenilendi."),
        "PROJECT_CREATED" => ("Project created.", "Proje oluşturuldu."),
        "PROJECT_MESSAGE_CREATED" => ("Project message added.", "Proje mesajı eklendi."),
        "PROJECT_SOURCE_EDITED" => ("Project message updated.", "Proje mesajı güncellendi."),
        "PROJECT_WORKSPACE_LINKED" => ("Worktree linked.", "Çalışma ağacı bağlandı."),
        "PROJECT_WORKSPACE_UNLINKED" => (
            "Worktree unlinked.",
            "Çalışma ağacının bağlantısı kaldırıldı.",
        ),
        "DESKTOP_LANGUAGE_SET" => ("Language preference saved.", "Dil tercihi kaydedildi."),
        "INVALID_DESKTOP_LANGUAGE" => {
            ("Choose Turkish or English.", "Türkçe veya İngilizce seçin.")
        }
        "STALE_PROJECT_REVISION"
        | "STALE_SOURCE_REVISION"
        | "STALE_ITEM_REVISION"
        | "STALE_RUN_REVISION"
        | "STALE_TEAM_REVISION"
        | "STALE_WORKSPACE_REVISION" => (
            "This item changed. Refresh and try again.",
            "Bu kayıt değişti. Yenileyip tekrar deneyin.",
        ),
        "PROJECT_NOT_FOUND" => ("Project no longer exists.", "Proje artık bulunamıyor."),
        "SOURCE_NOT_FOUND" => ("Source no longer exists.", "Kaynak artık bulunamıyor."),
        "SOURCE_IMMUTABLE" => (
            "Captured sources cannot be edited.",
            "Kaydedilmiş kaynaklar düzenlenemez.",
        ),
        "SOURCE_NOT_IN_PROJECT" => (
            "This source belongs to another project.",
            "Bu kaynak başka bir projeye ait.",
        ),
        "WORKSPACE_ALREADY_LINKED" | "WORKSPACE_LINKED_TO_OTHER_PROJECT" => (
            "This worktree is already linked to a project.",
            "Bu çalışma ağacı zaten bir projeye bağlı.",
        ),
        "WORKSPACE_NOT_LINKED" => (
            "This worktree is not linked to the project.",
            "Bu çalışma ağacı projeye bağlı değil.",
        ),
        "WORKSPACE_REGISTERED" => ("Folder added.", "Klasör eklendi."),
        "WORKSPACE_REMOVED" => (
            "Worktree registration removed.",
            "Çalışma ağacı kaydı kaldırıldı.",
        ),
        "WORKSPACE_ALREADY_REGISTERED" => (
            "This worktree is already registered.",
            "Bu çalışma ağacı zaten kayıtlı.",
        ),
        "WORKSPACE_IN_USE" => (
            "This worktree still has active work.",
            "Bu çalışma ağacında aktif işler var.",
        ),
        "WORKSPACE_NOT_FOUND" | "WORKSPACE_NOT_REGISTERED" => (
            "Worktree is no longer registered.",
            "Çalışma ağacı artık kayıtlı değil.",
        ),
        "WORKSPACE_SELECTION_CONFLICT" => (
            "Choose a registered worktree or a path, not both.",
            "Kayıtlı çalışma ağacı veya yol seçin; ikisini birlikte seçmeyin.",
        ),
        "WORK_ENQUEUED" => ("Task added to the queue.", "İş kuyruğa eklendi."),
        "WORK_STARTED" => ("Task started.", "İş başlatıldı."),
        "WORK_REMOVED" => ("Task removed.", "İş kaldırıldı."),
        "RUN_CANCELLED" | "AGENT_CANCELLED" => ("Run cancelled.", "Çalıştırma iptal edildi."),
        "RUN_CANCEL_PENDING" => ("Cancellation is pending.", "İptal işlemi bekliyor."),
        "WORK_ITEM_NOT_FOUND" | "RUN_NOT_FOUND" => (
            "The selected work no longer exists.",
            "Seçilen iş artık bulunamıyor.",
        ),
        "RUN_NOT_ACTIVE" | "WORK_ITEM_NOT_QUEUED" | "WORK_RETRY_UNAVAILABLE" => (
            "This action is unavailable in the current state.",
            "Bu işlem mevcut durumda kullanılamıyor.",
        ),
        "PROVIDER_CONTROL_UNAVAILABLE" => (
            "Provider control is unavailable. Refresh providers.",
            "Sağlayıcı kontrolü kullanılamıyor. Sağlayıcıları yenileyin.",
        ),
        "PROVIDER_CONTROL_FAILED" => (
            "Provider control failed.",
            "Sağlayıcı kontrolü başarısız oldu.",
        ),
        "RUN_SESSION_LOST" | "CODEX_SESSION_LOST" => (
            "The provider session was lost.",
            "Sağlayıcı oturumu kayboldu.",
        ),
        "OWNER_COMMAND_OUTCOME_UNKNOWN" | "START_OUTCOME_UNKNOWN" => (
            "The action outcome is uncertain. Review receipts and provider status before taking another action.",
            "İşlemin sonucu belirsiz. Yeni bir işlem başlatmadan önce sonuç kayıtlarını ve sağlayıcı durumunu kontrol edin.",
        ),
        "TEAM_CREATED" => ("Team plan created.", "Takım planı oluşturuldu."),
        "TEAM_STARTED" => ("Team started.", "Takım başlatıldı."),
        "TEAM_UPDATED" => ("Team updated.", "Takım güncellendi."),
        "TEAM_CANCELLED" => ("Team cancelled.", "Takım iptal edildi."),
        "QUESTION_ANSWERED" => ("Answer recorded.", "Yanıt kaydedildi."),
        "DECISION_APPROVED" => ("Decision approved.", "Karar onaylandı."),
        "DECISION_REJECTED" => ("Decision rejected.", "Karar reddedildi."),
        "TEAM_NOT_FOUND" | "AGENT_NOT_FOUND" => (
            "The selected team or agent no longer exists.",
            "Seçilen takım veya ajan artık bulunamıyor.",
        ),
        "TEAM_ACTION_UNAVAILABLE" | "TEAM_INVALID_ACTION" => (
            "This team action is unavailable in the current state.",
            "Bu takım işlemi mevcut durumda kullanılamıyor.",
        ),
        "PACK_INSTALLED" => ("Pack installed.", "Paket kuruldu."),
        "PACK_REMOVED" => ("Pack removed.", "Paket kaldırıldı."),
        "BUILT_IN_PACK" => (
            "Built-in packs cannot be removed.",
            "Yerleşik paketler kaldırılamaz.",
        ),
        "PACK_NOT_INSTALLED" => (
            "This pack is no longer installed.",
            "Bu paket artık kurulu değil.",
        ),
        "PACK_VERSION_MUST_INCREASE" => (
            "Use a newer version of this pack.",
            "Bu paketin daha yeni bir sürümünü kullanın.",
        ),
        "COMMAND_EXPIRED" => (
            "This action expired. Submit it again.",
            "İşlemin süresi doldu. Yeniden gönderin.",
        ),
        "IDEMPOTENCY_KEY_REUSED" => (
            "This command identity was used for a different action.",
            "Bu komut kimliği farklı bir işlem için kullanılmış.",
        ),
        "HOST_CLOSING" => ("Patchfleet is closing.", "Patchfleet kapanıyor."),
        "HOST_ACTION_FAILED" => ("The local action failed.", "Yerel işlem başarısız oldu."),
        "HOST_STATE_UNAVAILABLE" => ("Local state is unavailable.", "Yerel durum kullanılamıyor."),
        "LOCAL_WRITER_ALREADY_RUNNING" => (
            "Another Patchfleet instance owns this data directory.",
            "Bu veri dizinini başka bir Patchfleet kullanıyor.",
        ),
        "LOCAL_WRITER_RECOVERY_REQUIRED" | "LOCAL_STORAGE_EXTERNAL_CHANGE" => (
            "Local storage needs recovery. Back it up before repairing it.",
            "Yerel depolama kurtarılmalı. Onarmadan önce yedekleyin.",
        ),
        "CLOUD_RECOVERY_REQUIRED" | "CLOUD_STATE_INVALID" => (
            "Cloud settings need local recovery.",
            "Bulut ayarları yerelde kurtarılmalı.",
        ),
        "CLOUD_UNPAIRED" => ("Cloud is disconnected.", "Bulut bağlantısı kesildi."),
        "CLOUD_ALREADY_PAIRED" => (
            "This host is already paired.",
            "Bu ana makine zaten eşleştirilmiş.",
        ),
        "CLOUD_AUTH_REJECTED" => (
            "Cloud access was rejected. Pair again.",
            "Bulut erişimi reddedildi. Yeniden eşleştirin.",
        ),
        "CLOUD_URL_INVALID" => (
            "Enter a valid Cloud URL.",
            "Geçerli bir bulut adresi girin.",
        ),
        "CLOUD_SYNC_FAILED" | "CLOUD_UNAVAILABLE" => (
            "Cloud sync is unavailable. Local work continues.",
            "Bulut eşitlemesi kullanılamıyor. Yerel çalışma devam eder.",
        ),
        "INVALID_PAYLOAD"
        | "INVALID_COMMAND_TIME"
        | "INVALID_PROJECT_QUERY"
        | "INVALID_PROJECT_TEXT" => (
            "Check the form values and try again.",
            "Formdaki değerleri kontrol edip yeniden deneyin.",
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_defaults_and_unknown_copy_are_explicit() {
        assert_eq!(
            Language::from_snapshot(&serde_json::json!({})),
            Language::Tr
        );
        assert_eq!(tr(Language::Tr, "Projects"), "Projeler");
        assert_eq!(tr(Language::En, "Projects"), "Projects");
        assert_eq!(
            tr(Language::Tr, "Custom owner content"),
            "Custom owner content"
        );
        assert_eq!(tr(Language::Tr, "PROJECT CREATED"), "Proje oluşturuldu.");
        assert_eq!(tr(Language::En, "PROJECT_CREATED"), "Project created.");
    }
    #[test]
    fn formatting_does_not_translate_or_reinterpret_inserted_owner_text() {
        assert_eq!(
            format(
                Language::Tr,
                "Author {0} · updated {1}",
                &["Projects {1}", "2026-10-14"]
            ),
            "Yazan Projects {1} · güncelleme 2026-10-14"
        );
    }
}
