# Moxcrew UI/UX başvuru kuralları

Araştırma ve kaynak erişim tarihi: **2026-10-25**.

Bu belge, sonraki arayüz çalışmalarının tasarım ve inceleme referansıdır.
Araştırmayı, mevcut uygulamanın teşhisini ve uygulanacak tasarım hedeflerini
bir araya getirir. Buradaki hedeflerin yazılması, ekranların düzeltildiği veya
erişilebilirlik uygunluğu elde edildiği anlamına gelmez.

Uygulama takibi: [Task0053](../../docs/plans/0053-apply-ux-rules-and-chips.md)
ve [uygulama kanıtları/durumu](../../docs/state/ux-rules-implementation.md).
Aşağıdaki kaynak teşhisi araştırma anının kaydıdır; hangi düzeltmelerin
gerçekleştiği uygulama durumundan okunur.

Mevcut ölçü ve bileşenlerin kaynağı [UI-KIT.md](UI-KIT.md); ürün yetki ve durum
sözleşmelerinin kaynağı kabul edilmiş ADR'lerdir. Özellikle
[ADR0047](../../docs/decisions/0047-chat-centered-project-context.md),
[ADR0048](../../docs/decisions/0048-conversation-first-orchestration.md),
[ADR0049](../../docs/decisions/0049-automatic-planning-connection.md) ve
[ADR0050](../../docs/decisions/0050-local-product-capabilities.md) geçerlidir.
Bu referans çalışma başlatma, dış işlem onayı veya Cloud yetkisini genişletmez.
Uygulama değişiklikleri ayrıca sınırları belirli, onaylı bir görev kartıyla yapılır.

## 1. Hangi kaynak neyi belirliyor?

Tek bir evrensel "güzel arayüz standardı" yoktur. Kaynak türleri birbirine
karıştırılmamalıdır:

| Tür | Başvuru | Moxcrew için anlamı |
|---|---|---|
| Normatif web erişilebilirlik standardı | [WCAG 2.2](https://www.w3.org/TR/WCAG22/) | Ölçülebilir başarı kriterleri vardır; tek başına görsel tasarım sistemi değildir. |
| Web dışı yazılıma uyarlama rehberi | [WCAG2ICT](https://www.w3.org/TR/wcag2ict/) | Native yazılıma yorumlamada kullanılır; W3C Group Note bilgilendiricidir, yeni bir normatif uygunluk standardı değildir. |
| İnsan odaklı tasarım ve etkileşim standartları | [ISO 9241-210:2019](https://www.iso.org/standard/77520.html), [ISO 9241-110:2020](https://www.iso.org/standard/75258.html) | Süreç ve etkileşim çerçevesi; hazır piksel ölçüsü veya marka estetiği tarifi değildir. Bu araştırmada resmi özetler okundu; ücretli tam metinler incelenmedi. |
| Platform/bileşen rehberi | Apple HIG, Fluent 2, Material | Belirli bağlamlarda davranış ve yerleşim örnekleri. Bir platformun kuralını bütün ekranlara genellemeyiz. |
| Kullanılabilirlik araştırması/ilkeleri | Nielsen Norman Group, Microsoft Research | Tasarım hipotezlerini ve değerlendirmeyi besler; sertifika veya her ürün için değişmez yerleşim değildir. |
| Moxcrew tasarım kararı | Bu belgedeki uygulama kuralları | Kaynaklardan ve ürünün işinden türetilmiş hedeflerdir; kullanıcılarla doğrulanır. |

Örneğin 8'lik boşluk sistemi, yuvarlak köşeler ve gönderme düğmesinin sağda
olması Moxcrew tercihidir. Bunları "WCAG/ISO böyle istiyor" diye sunmayız.

## 2. Mevcut sorunun teşhisi

İncelenen çalışma ağacı: `5e07ee1`; son runtime değişikliği: `072f371`.
Aşağıdaki bulgular kaynak incelemesidir. Son madde ayrıca önceki yerel test
yakalamalarında görülür; bunlar yeni bir canlı kullanıcı oturumu değildir.

| Bulgu | Kaynak | Kullanıcıya etkisi / tasarım hedefi |
|---|---|---|
| Temel buton varyantları mevcut; anlamlı chip ve birleşik akış bileşenleri eksik. | [studio.rs](../../crates/patchfleet-desktop/src/studio.rs), `ButtonKind`, `kit_button`, `view_switch` | Her şeyi normal butona dönüştürmek yerine eylem, seçim, durum ve gezinme ayrılmalı. |
| Ekler ve çıktı seçimi aynı 76 birimlik kaydırma alanına giriyor. | [chat.rs](../../crates/patchfleet-desktop/src/chat.rs), `chat_composer`; [capabilities.rs](../../crates/patchfleet-desktop/src/capabilities.rs), `attachment_controls` | Dosya satırları ve ayrı kaldır düğmeleri editörü şişiriyor; gerekli seçim açıklamaları kırpılabiliyor. Kompakt ek chip'leri ve ayrı, görünür çakışma çözümü gerekiyor. |
| Ana composer eylemi, seçime göre Send / Prepare task / Prepare revision oluyor; eylem satırı soldan başlıyor. | [chat.rs](../../crates/patchfleet-desktop/src/chat.rs), `chat_composer` | Konum tek başına hata değildir; kullanıcının hangi modda olduğu ve tıklamanın sonucu açık değilse hata olur. Gönderme ile görev hazırlama farklı bağlamlarda anlatılmalı. |
| Genel busy durumu bütün Create yüzeyini devre dışı bırakıyor. | [ui.rs](../../crates/patchfleet-desktop/src/ui.rs), `create_studio` çağrısını saran `add_enabled_ui(!busy, ...)` | Okuma/işlem sürerken yerel taslak düzenleme de kilitlenebiliyor. Kilit, çakışan komutla sınırlanmalı. |
| Project bağlamı 1280 eşiğinde sabit 600 birimlik, yeniden boyutlandırılamayan panele dönüşüyor. | [chat.rs](../../crates/patchfleet-desktop/src/chat.rs), `project-context-panel` | Kalan sohbet alanının ihtiyacı gözetilmiyor; yan panel görünürlüğü pencere ve içerik genişliğiyle birlikte ele alınmalı. |
| Brand kaydetme eylemi alanlardan önce çiziliyor. | [capabilities.rs](../../crates/patchfleet-desktop/src/capabilities.rs), `brand_form` | Kullanıcı önce neyi düzenlediğini, sonra nasıl kaydedeceğini görmeli; kaydetme formun sabit eylem alanına taşınmalı. |
| Geometri, komut doğruluğu ve paket kontrolleri başarılı olsa da anlaşılır hiyerarşi kanıtlanmış değil. | [UI kit geçmiş durumu](../../docs/state/ui-kit-screen-migration.md), [özellik doğrulaması](../../docs/state/eleven-product-capabilities.md) | Mevcut minimum pencere ekler yakalamasında ikinci satırın kısmen kesilmesi kabul edilmiş. Yeni kabul kuralı: temel bilgi/eylem kırpılıyorsa görsel inceleme geçemez. |

Temel süreç hatası: önce özellikleri ayrı kontroller olarak eklemek, sonra
"ekrana sığıyor mu?" kontrolünü ürün tasarımı doğrulaması saymak. Aynı renk,
padding ve radius kullanmak, doğru bilgi mimarisi veya anlaşılır bir akış
oluşturmaz. Tasarım değerlendirmesi görev ve kullanıcı beklentisi üzerinden
yapılmalıdır.

## 3. Rust mı sınırlıyor?

**TECH-01 — Dil ile arayüz kütüphanesini ayır.** Rust; chip, sekme, menü,
popover, bölümlü seçim veya animasyon kullanımını engellemez. Proje native
`eframe/egui 0.36.2` kullanıyor; bir web arayüzü veya otomatik AppKit/WinUI
bileşen koleksiyonu kullanmıyor.

[egui'nin kendi açıklaması](https://github.com/emilk/egui) özel widget ve çizimi
desteklediğini, native görünümü hedeflemediğini belirtir.
[Sürümle eşleşen Ui API'si](https://docs.rs/egui/0.36.2/egui/struct.Ui.html)
seçilebilir kontroller, özel etkileşim ve çizim olanakları sağlar. Dolayısıyla
chip yapılabilir. Hazır, tutarlı bir tüketici ürünü deneyimini kurmak için
bileşen davranışı, yerleşim ve semantiği bizim tamamlamamız gerekir.

**TECH-02 — Görüntüyü değil davranışı yeniden kullan.** Önce mevcut
`studio.rs` kontrollerini ve seçili durum bilgisini kullan. Dosya chip'i gibi
gerçek ihtiyaç için ortak bileşen eklenebilir; birkaç şekil çizmek için yeni
bağımlılık veya ikinci bir tasarım sistemi ekleme. Özel çizim; klavye, odak,
isim/rol/seçim durumu ve erişilebilirlik bilgisinin yerine geçmez.

**TECH-03 — Framework kararı ile tasarım açığını karıştırma.** Web bileşen
ekosisteminde birçok hazır örüntü bulunması işimizi kolaylaştırabilir. Bu,
web'de iyi tasarımın otomatik olduğu veya mevcut hataların Rust'tan kaynaklandığı
anlamına gelmez. Buradaki sorunları gidermek için framework göçü gerekliliği
gösterilmedi; native mimari kararı sürüyor. Eguili ürünün platform davranışları
ve erişilebilirliğini ayrıca tasarlama/test etme maliyeti gerçektir.

## 4. Bilgi mimarisi ve ana akış

[Aşamalı açıklama](https://www.nngroup.com/articles/progressive-disclosure/)
seyrek kullanılan ayrıntıları ikincil yüzeylere almayı destekler; sık kullanılan
işleri görünmez menülere saklamak için gerekçe değildir. Moxcrew'e uyarlamamız:

**FLOW-01 — Ana yüzey kullanıcının amacıdır.** İlk ekranda mesaj alanı,
gönderme ve dosya ekleme bulunur. Kullanıcı önce Project, sağlayıcı, ability,
reçete, takım veya çalışma modu formu doldurmaz. Örnek istekler isteğe bağlı
başlangıç önerileridir; boş duruma aitler, her mesajda tekrarlanmazlar.

**FLOW-02 — Bir ana gezinme, bir bağlam.** Sohbet ana çalışma alanıdır.
Takım ve Çıktılar aynı Project'in bağlamlarıdır. Aynı planı, aynı başlatma
eylemini veya aynı dosya listesini hem sohbet içinde tam boy hem yan panelde
tam boy çoğaltma. Yan panel açılınca sohbet özeti kalabilir. Proje değişimi,
seçimler ve görünür başlıklar her zaman aynı kapsamı göstermelidir.

**FLOW-03 — Normal yol aşağıdaki sırayı izler.** Bunlar hedef etkileşimlerdir;
mevcut uygulamadaki bütün girişlerin bu hedefe ulaştığı iddia edilmez.

| Aşama | Kullanıcının gördüğü / yaptığı | Moxcrew'ün sorumluluğu |
|---|---|---|
| İstek | Amacını yazar, isterse dosya ekler; **Gönder**. | Taslağı koru. İlk açık gönderimde gerekli Project'i mevcut kesin receipt sözleşmesiyle oluştur. |
| Bağlantı | Hazır bağlantı varsa ayrıca seçim yapmaz. Yoksa taslağın yanında **Bağlantı kur** görür. | Kullanılabilir sağlayıcıyı mevcut otomatik sırayla belirle; isteğe bağlı override sun. Açıkça seçilmiş ama kullanılamayan sağlayıcıyı sessizce değiştirme. Gerçek bağlantı yokken Mox'un isteği anladığını uydurma. |
| Anlama | Gerekliyse Mox'un kısa sorusuna aynı mesaj alanından yanıt verir. | Yalnızca sonucu değiştiren eksik bilgiyi sor; teknik kurulum anketi açma. Gerçek sağlayıcı yanıtını göster. |
| Takım önerisi | Amaç, önerilen roller ve beklenen çıktıları tek kısa plan kartında görür. | Ayrıntılı roster/ajan talimatlarını isteğe bağlı aç. Sonraki eylem **Takımı hazırla**. |
| Hazırlanmış iş | Hazır kapsamı ve varsa engeli görür; **Başlat**. | Hazırlama ile çalıştırmayı ayrı tut. Göndermeyi veya bir öneri chip'ini tıklamayı çalışma izni sayma. |
| Çalışma | Gerçek durum, son anlamlı gelişme ve gerektiğinde **Durdur**. | Ajan konuşmaları/detaylı loglar ikincil incelemede. Sahte yüzde, süre veya tamamlanma gösterme. |
| Sonuç | Gerçek çıktıyı açar/önizler; gerekirse sürümünü seçer. | Dosyanın türünü, kaynağını ve durumunu anlaşılır göster. İçerik henüz yoksa başarı kartı gösterme. |
| Revizyon | Sayfa/bölge/zaman aralığı seçimi mesaj alanında chip olarak görünür; değişikliği yazar. | Yeni sürüm için açık **Düzenlemeyi hazırla** bağlamı; çalıştırma yine açık başlatma adımıdır. Orijinal ve kaynak sürümü korunur. |

**FLOW-04 — Gelişmiş işler bağlamsal açılır.** Brand/skill/trigger gibi ayarlar
Project araçlarında; ilgili çıktıdaki düzenleme o çıktının yanında bulunur.
Sohbet her özelliğin kalıcı kontrol paneline dönüşmez. Eksik yerel motorun
kurulumu ilgili işte açıklanır. Tetikleyici kurma/etkinleştirme gibi gelecekte
çalışma izni veren işlemler açık kapsam ve sınırlarla sunulur.

**FLOW-05 — Project kapsamı görünürdür.** Brand ve kabul edilmiş talimatlar
Project'e aittir. Genel uygulama ayarları başka bir kapsamdır. Tasarım bu iki
kapsamı aynı isimsiz ayar paneline koymaz. Eski bir Project'e ait geç gelen
yanıt yeni Project'in taslağına veya paneline uygulanmaz.

## 5. Hangi bileşeni ne için kullanacağız?

[Material chip sınıfları](https://material-web.dev/components/chip/) assist,
filter, input ve suggestion kullanımını ayırır. Aşağıdaki Moxcrew eşlemesi
bizim tasarım hedefimizdir; bu rollerin hepsi normal buton görünümü almak
zorunda değildir.

| Bileşen | İşlev | Moxcrew örneği / sınırı |
|---|---|---|
| Eylem butonu | Bir işlemi gerçekleştirir. | Gönder, Kaydet, Başlat. Fiil ve sonuç açık. |
| Sekme | Aynı düzeydeki kalıcı bağlamlar arasında gezinir. | Sohbet / Takım / Çıktılar. Seçim kaydetmez veya çalışma başlatmaz. |
| Bölümlü kontrol | Aynı içerikte birbirini dışlayan görünüm seçer. | Liste / Önizleme. Görünüm tercihini çalışma komutuna dönüştürme. |
| Filter chip | Görünen kümede filtre açar/kapatır. | Çıktılarda Görsel / Belge / Video; seçili durum okunabilir olmalı. |
| Input chip | Eklenmiş nesne veya seçili bağlamdır. | `brief.pdf ×`, `Sayfa 3 ×`, `00:12–00:18 ×`. Kaldırma taslaktan çıkarır, orijinal dosyayı silmez. |
| Suggestion chip | İsteğe bağlı başlangıç/yanıt önerisidir. | “Sunum hazırla”. Varsayılan olarak taslağa ekler; kendiliğinden göndermez veya başlatmaz. |
| Assist chip | Bağlamla ilgili küçük yardımcı eylemdir. | Seçili metni özetleme isteğini hazırlamak. Yan etkili işi tek tıkta sessizce çalıştırmaz. |
| Durum rozeti | Bir durumu bildirir. | Çalışıyor, Hazır, Bağlı değil. Etkileşimsizse tıklanabilir kontrol gibi davranmaz. |
| Checkbox | Bağımsız, çoklu seçim. | Pakete dahil edilecek dosyalar. |
| Radio/select | Tek form değeri seçimi. | İsteğe bağlı bağlantı override'ı; az seçenek radio, uzun liste select. |
| Switch | Anında uygulanan açık/kapalı tercih. | Uygun uygulama ayarı. Çalıştırma yetkisi veren tetikleyici etkinleştirmesini sıradan görsel tercih gibi sunma. |
| Menü/popover | Seyrek eylem veya kısa bağlamsal seçim. | Yeniden adlandır, teknik bağlantı ayrıntısı. Ana Gönder/Başlat işini saklama. |
| Açılır bölüm | İsteğe bağlı ayrıntıyı gösterir. | Ajan talimatları, gelişmiş parametreler. Zorunlu hata çözümü kapalı bölümde kalamaz. |
| Dialog | Odak gerektiren sınırlı karar veya düzenleme. | Kapsamı belli dış işlem incelemesi; normal durum bilgisini modal yapma. |

**COMP-01 — Chip küçük etiket çizmekten ibaret değildir.** Etkileşimli chip'te
varsayılan, hover, klavye odağı, seçili ve devre dışı durumları ayrılır; gereken
yerde yükleme/hata bilgisi eklenir. Kaldırma eyleminin adı örneğin
“brief.pdf ekini kaldır” olur. Gövde ve kaldırma ayrı eylemse odakları ve
tıklama alanları çakışmaz. Sadece rengi değiştirerek seçim anlatılmaz.

**COMP-02 — Ekler mesaj alanını işgal etmez.** Dosya adı/türü ve hazır/hata
durumu görünürdür. Uzun ad kısaltılabilir; tam ad odak/ayrıntı üzerinden de
erişilebilir olur. Fazla dosyalar `+N dosya` ile tam listeye açılır; sayı gerçek
taslağı yansıtır. Kaçının satıra sığacağı kullanılabilir genişlikten ölçülür.
Dosya listesi kaydırılabilir ama mesaj editörü, gönderme ve zorunlu çakışma
çözümü o listenin kırpma alanının dışında kalır.

**COMP-03 — Renk ve şekil semantiği tutarlıdır.** Primary, secondary, ghost,
danger rolleri shared kit'ten gelir. Aynı anlama aynı etiket ve simge kullanılır.
İkon butonlarda erişilebilir isim; yabancı/seyrek simgelerde görünür açıklama
bulunur. Her şeyi chip yapmak da her şeyi buton yapmak kadar hatalıdır.

## 6. Butonların yeri ve önceliği

Apple [alert rehberi](https://developer.apple.com/design/human-interface-guidelines/alerts)
yatay onay sıralarında varsayılan eylemi trailing tarafta konumlandırır.
[Fluent button rehberi](https://fluent2.microsoft.design/components/web/react/core/button/usage)
kendi yerleşimlerinde primary için üst/başlangıç tarafını önerir. Bunlar farklı
bağlamlardır. Moxcrew'ün bütün butonlarını rastgele sağa taşıyarak tutarlılık
elde edilmez.

**ACT-01 — Öncelik bir görev bağlamında kurulur.** Bir kart/formun belirgin
sonraki eylemi vardır. Farklı bağlamlar aynı pencerede bulunabilir; hepsi aynı
anda vurgu rengiyle yarışmaz. İkincil eylem ana eylemle aynı görsel ağırlığı
almaz. Hazırla ile Başlat aynı aşamanın iki eşdeğer düğmesi değildir.

| Bağlam | Moxcrew hedef yerleşimi | Açıklama |
|---|---|---|
| Mesaj alanı | Ekler/seçim üstte; editör ortada; dosya ekleme başlangıçta, Gönder bitişte | Türkçe/İngilizcede ekle solda, gönder sağda. Bağlantı override'ı ikincil menüdedir. Konum durum değişince sıçramaz. |
| Takım önerisi | Önce amaç ve çıktı, sonra rol özeti; footer'da ayrıntı/düzenle ve belirgin Hazırla | Hazırlanmış durumda aynı karar alanı Başlat'ı sunar; eski aşamanın eylemi eşit CTA olarak kalmaz. |
| Düzenleme formu | Alanlar içerikte, Kaydet/İptal sabit footer'da | Uzun form kaydırılır; eylem footer'ı içeriği örtmez. Değişmemiş formda kaydetme durumu ve dirty durumu anlaşılırdır. |
| Çıktı önizleme | Başlık/tür, içerik, yakınında bağlamsal seçim/düzenleme; seyrek eylemler menüde | Düzenleme seçili sürüme bağlıdır. Dışa aktarma/açma mevcut doğrulanmış dosya sözleşmesini kullanır. |
| Kritik onay | Sonuç ve etkilenen nesne açık; platformun beklenen onay/iptal sırası | “Tamam” yerine “Dosyayı sil” gibi sonuç belirten ad. Yıkıcı eylem Moxcrew'de varsayılan Enter eylemi yapılmaz. |
| Liste satırı | Birincil nesne adı sabit hizalı; az sayıdaki bağlamsal eylem bitişte | Her satırda üç büyük butonla içeriği bastırma; seyrek eylemleri menüye al. |

**ACT-02 — Sık eylem saklanmaz, tehlikeli eylem ayrılır.** Dosya ekleme ana
girişin parçasıdır; yalnız “Diğer” altında keşfedilmesi beklenmez. Silme veya
çalışmayı durdurma, normal görünüm değiştirme yanında yanlışlıkla seçilecek
vurgu/default davranışı almaz. Geri alınabilir taslak chip kaldırma için sürekli
onay sorulmaz; geri alma veya tekrar ekleme yolu sağlanır.

**ACT-03 — Disabled, açıklama yerine geçmez.** İşlem neden yapılamıyor ve
nasıl açılacağı ilgili yerde okunur. Zorunlu bilgi yalnız disabled düğmenin
hover tooltip'ine bırakılmaz. İşlem sırasında yalnız çakışan eylemler kilitlenir;
kullanıcı taslağını düzenlemeye ve izin verilen incelemeye devam edebilir.

## 7. Görsel düzen, yoğunluk ve masaüstü davranışı

**VIS-01 — Önce hiyerarşi, sonra dekor.** Başlık işin ne olduğunu, ana içerik
istenen kararı, footer sonraki eylemi anlatır. Büyük boş kartlar, iç içe çerçeveler,
her alanda glow veya bütün metinlerde aynı ağırlık hiyerarşi kurmaz. Hafif
fütüristik kimlik; Mox, semantik kobalt vurgusu, kontrollü yüzey derinliği ve
işi açıklayan kısa geçişlerle korunur.

**VIS-02 — Ölçüleri tek kaynaktan kullan.** Mevcut kit: 4/8/12/16/24/32/48
boşluk ölçeği; 36 regular, 32 compact kontrol yüksekliği; 14 body, 12 metadata.
Bunlar mevcut uygulama değerleridir, evrensel erişilebilirlik zorunluluğu
değildir. Yoğun masaüstü varyantı gerekiyorsa gerçek içerik ve hit target ile
prototiplenir; bu belge token'ları sessizce değiştirmez. Ana karar/hata bilgisini
küçük ve soluk metadata biçimine itme.

**VIS-03 — Görsel alan ile hit target ayrılabilir.** Kompakt chip ve simge
görünümü, yeterli ve çakışmayan etkileşim alanına sahip olabilir. Eguili mantıksal
birimler, Retina fiziksel pikselleri, CSS px ve Android dp doğrudan eşdeğer
ölçüler gibi değerlendirilmez. Padding aynı anda hem okunabilirlik hem görev
yoğunluğu açısından incelenir; kısa pencereyi çözmek için rastgele küçültülmez.

**VIS-04 — Uyarlama içerik ihtiyacından çıkar.** Sohbetin okunabilir genişliği,
yan panelin asgari içeriği, sidebar ve aralıklar birlikte ölçülür. İkisi
sığmıyorsa yan panel tek bağlam yüzeyi olur; dönüşte taslak/seçim korunur.
Geniş pencerede yeniden boyutlandırılabilen panel değerlendirilebilir. Sabit
eşikte sabit 600 alan ayırmak kabul ölçütü değildir.

**VIS-05 — Kaydırma niyetlidir.** Sohbet geçmişi ve uzun inceleme listesi
kaydırılır. Composer içinde birkaç satırlık iç içe scroll ile temel eylem/hata
gizlenmez. Pencere küçüldüğünde footer içeriği örtmez. 800×560 mantıksal minimum
pencere, normal/geniş boyut, %125/%150 ölçek, uzun Türkçe etiketler ve panel
açma/kapama geçişleri ayrı kontrol edilir; fiziksel ekran görüntüsü boyutu
mantıksal viewport sanılmaz.

**VIS-06 — Masaüstü alışkanlıkları korunur.** Cmd/Ctrl ayrımı, seçme/kopyalama,
metin undo/redo, pencere yeniden boyutlandırma, sürükleyip dosya bırakma ve
görünür klavye odağı beklenen biçimde işler. Mevcut Cmd/Ctrl+Enter gönderme
kısayolu görünür ipucuyla korunur; Enter davranışını değiştirmek ayrıca test
edilen bir etkileşim kararıdır. IME ile metin onaylamak mesaj göndermemelidir.

## 8. Erişilebilirlik kabul hedefleri

Aşağıdaki WCAG numaraları kaynak kriterlerini gösterir. Native Moxcrew için
WCAG2ICT yorumu ve gerçek platform yardımcı teknoloji testi gerekir; sayıları
kopyalamak veya AccessKit feature'ını açmak uygunluk kanıtı değildir.

| ID | Kaynak kriteri ve sınır | Moxcrew kontrolü |
|---|---|---|
| A11Y-01 | [1.4.3 AA](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html): normal metin 4.5:1, kriterin büyük metin tanımında 3:1; istisnalar vardır. | Gerçek tema çiftlerini ölç. Başlığı büyük göründüğü için otomatik büyük metin istisnasına sokma; CSS pt tanımını egui ölçüsüyle karıştırma. |
| A11Y-02 | [1.4.11 AA](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): gerekli kontrol/durum göstergelerinde bitişik renge karşı 3:1; her dekoratif kenarlık için değil. | Seçim, focus ve gerekli ikon ayırt edilir; yalnız renk veya hover ile anlam verme. |
| A11Y-03 | [2.5.8 AA](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum): 24×24 CSS px ve belirtilen istisnalar. | Bu sayı “her native buton 24/44/48 olmalıdır” diye çevrilmez. Native hit alanı, aralık ve yanlış tıklama riski birlikte ölçülür; chip kaldırma alanları çakışmaz. |
| A11Y-04 | [2.1.1 A](https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html): işlevler, yol bağımlı giriş istisnası dışında klavyeyle kullanılabilir. | Ek ekle/kaldır, filtre, menü, gönderme, plan inceleme, başlatma, önizleme ve revizyon fare gerektirmez. Görsel bölge seçiminin klavye karşılığı korunur. |
| A11Y-05 | [2.4.11 AA](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum): odaklı kontrol tamamen örtülmez. | Moxcrew daha sıkı hedef koyar: odaklanan kontrolün tamamı görünür olsun. Kit'in 2 birim focus halkası kendi tasarım tercihidir; bu ölçü tek başına AA kanıtı değildir. |
| A11Y-06 | [3.3.1 A](https://www.w3.org/WAI/WCAG22/Understanding/error-identification.html): hatalı alan ve hata metinle tanımlanır. | Alan etiketi kalır; placeholder tek açıklama olmaz. Düzeltme adımı yakında verilir; hata sonrası odak anlamlı yerde tutulur. |
| A11Y-07 | [4.1.3 AA](https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html): durum mesajları odağı taşımadan programatik belirlenebilir. | “Dosya eklendi”, “Hazır”, “Bağlantı başarısız” yardımcı teknolojiye ulaşır. Her ajan token'ı duyurulmaz, yeni cevapta kullanıcı odağı çalınmaz. |

**A11Y-08 — Bileşen semantiği elle doğrulanır.** İsim, rol, değer/seçili durum,
enabled durumu ve gruplama anlamlıdır. Menüler klavyeyle açılır/kapanır;
dialog kapanınca odak tetikleyiciye döner. Escape menü/dialog bağlamında
çalışır, gizli global görev iptali yapmaz. Özel çizilmiş bileşenlerde bu
semantik ayrıca sağlanır. VoiceOver/macOS, NVDA/Windows ve Linux'ta ilgili
erişilebilirlik altyapısıyla doğrulama yapılmadan bu platformlar için başarı
iddia edilmez.

**A11Y-09 — Hareket bilgiye hizmet eder.** Gerçek iş durumuna bağlı kısa
geçişler kullanılır; reduced-motion tercihiyle gereksiz hareket kaldırılır.
Yanıp sönen süsler, sürekli dolaşan ajan avatarları veya iş yapılmıyorken
çalışıyormuş gibi animasyonlar eklenmez. Önemli durum yalnız animasyonla anlatılmaz.

## 9. Durumlar, hatalar ve AI ile işbirliği

[Microsoft Human-AI rehberi](https://www.microsoft.com/en-us/research/?p=564561)
sistemin yapabildiklerini ve sınırlarını anlaşılır kılma; kullanıcı düzeltmesi,
belirsizliği çözme ve kontrolü destekleme ilkeleri sunar. Moxcrew'de bunların
somut karşılığı gerçek plan, açık çalışma sınırı, görünür seçim ve kurtarılabilir
taslaktır. [Kullanılabilirlik heuristikleri](https://www.nngroup.com/articles/ten-usability-heuristics/)
de durum görünürlüğü, tutarlılık, kontrol, hata önleme ve kurtarmayı değerlendirme
çerçevesidir; prototipin kullanıcıyla test edilmesinin yerine geçmez.

**STATE-01 — Kullanıcının anlayacağı durum ve sonraki eylem gösterilir.**
Aşağıdaki adlar sunum gruplarıdır; yeni canonical event veya backend enum'u
tanımlamaz. İlgili işlemin desteklediği gerçek durumlara eşlenir.

| Durum | Gösterilecek bilgi | Ana/bağlamsal eylem |
|---|---|---|
| Taslak | Amaç ve seçili dosya/çıktı bağlamı | Gönder veya açıkça adlandırılmış revizyon hazırlama |
| Bağlantı gerekli | Eksik bağlantı ve taslağın korunduğu | Bağlantı kur; isteğe bağlı mevcut bağlantıyı değiştir |
| Planlanıyor | Gerçek planlama işlemi ve varsa süre değil son olay | Destekleniyorsa iptal; yerel taslak düzenlenebilir |
| Yanıt gerekli | Sağlayıcının sonucu belirleyen sorusu | Aynı composer'dan yanıtla |
| Plan hazır | Amaç, roller, beklenen çıktılar | Takımı hazırla; ikincil düzenle/ayrıntı |
| Hazırlanıyor | Gerçek hazırlama durumu | Çakışan çift gönderimi engelle; inceleme açık |
| Hazırlanmış | Dondurulmuş kapsam ve gerçek engel/hazır bilgisi | Başlat; varsa engeli çöz |
| Çalışıyor / bekliyor | Son gerçek olay ve beklemenin nedeni | Desteklenen durdurma; ilgili ayrıntıyı aç |
| Durdurma istenmiş | İsteğin alındığı, tamamlanmanın henüz doğrulanmadığı | Tekrarlı durdurmayı engelle; gerçek sonucu bekle |
| Hata | Ne başarısız, ne korundu, nasıl düzeltilebilir | Bağlamlı düzelt/yeniden dene; hatayı sadece kapatma sunma |
| Sonuç belirsiz | İşlemin sonucunun doğrulanamadığı | Önce durumu doğrula; yan etkili işi körlemesine tekrarlama |
| Tamamlandı | Gerçek sonuç ve doğrulanmış çıktılar | Aç/önizle; ilgili çıktıdan düzenleme iste |
| İptal edildi | Gerçekte durdurulan kapsam ve varsa oluşmuş sonuç | Taslağı/sınırlı sonucu incele; yeni iş açık kullanıcı eylemiyle |

**STATE-02 — Geri bildirim gerçek işlemle ilişkilidir.** Eylem alındığında
görsel/erişilebilir geri bildirim verilir. Alındı, uygulandı ve tamamlandı aynı
şey değildir. Bir alanı kaydetmek için bütün sohbeti kilitleme. Kesin yüzde
hesaplanmıyorsa uydurma progress bar yerine doğru aşamayı göster.

**STATE-03 — Kurtarma veri kaybettirmez.** Bağlantı/engine hatası, panel kapama,
Project geçişi veya geç gelen yanıt yazılmış isteği silmez. Silinmiş/eskimiş
çıktı seçimi sessizce başka dosyayla değiştirilmez. Retry mevcut idempotency ve
receipt kurallarına uyar. Dış işlem sonucu belirsizken otomatik tekrar yapılmaz.

**STATE-04 — Uyarı şiddeti orantılıdır.** Rutin kaydetme ve dosya ekleme için
modal başarı penceresi açma. Alan hatası alanın yanında, bağlantı sorunu bağlantı
bağlamında, engelleyici karar gerektiği yerde görünür. Gizli hata + gri buton
bir kurtarma akışı değildir. Gizlilik için ayrıntı kısaltılabilir; gerekli
çözüm ve güvenli teşhis yolu yine sunulur.

## 10. Kabul ve tasarım çalışma yöntemi

**QA-01 — Önce tek bir uçtan uca akışı doğrula.** Boş durum → istek → bağlantı
veya açıklama → plan → hazırlama → başlatma → sonuç → revizyon durumlarını
aynı taslakla tasarla. Ardından ortak bileşeni diğer ekranlara uygula. Her
özelliğe ayrı form ekleyip daha sonra genel padding düzeltmesi yapma.

**QA-02 — Gerçek içerik matrisi kullan.** TR/EN, açık/koyu tema; minimum/normal/
geniş pencere; ölçek değişimi; uzun dosya/Project adları; 0/1/12 ek; seçili
revizyon ile ek çakışması; hazır/eksik sağlayıcı; eksik engine; uzun konuşma;
bekleme/hata/belirsiz sonuç; panel ve Project geçişi birlikte değerlendirilir.
Sadece boş fixture veya kısa İngilizce etiket yeterli değildir.

**QA-03 — Altı kritik işi yönlendirmeden denetle.**

1. Yeni, geliştirici olmayan kullanıcı amacını yazıp ilk planına ulaşabilmeli.
2. Dosyaları ekleyip birini kaldırabilmeli; neyin gönderileceğini söyleyebilmeli.
3. Kullanılamayan bağlantıyı taslağını kaybetmeden düzeltebilmeli.
4. Takım önerisini anlayıp hazırlama ile çalıştırma arasındaki farkı görebilmeli.
5. Gerçek çıktıyı önizleyip doğru sürüme değişiklik isteyebilmeli.
6. Hata veya Project değişimi sonrasında işini bulup güvenle sürdürebilmeli.

Moxcrew için başlangıç çalışması: farklı iş amaçları olan 5–8 kişiyle küçük,
biçimlendirici kullanılabilirlik oturumları. Bu sayı proje araştırma tercihidir;
evrensel standart veya istatistiksel temsil iddiası değildir. Görev tamamlama,
kritik hata, yanlış tıklama, geri dönüş, yardım ihtiyacı ve sonraki adımı doğru
anlatabilme kaydedilir. İyi görünen ekran yakalaması bu kanıtın yerine geçmez.

**QA-04 — Şunlar arayüz kabulünü durdurur:**

- Temel eylem, zorunlu hata çözümü veya seçili bağlamın kırpılması/örtülmesi.
- Açıklamasız disabled ana eylem; ilgisiz işlem yüzünden bütün yerel editörün kilitlenmesi.
- Aynı iş için birbiriyle yarışan ana CTA'lar veya görünmeyen mod değişikliği.
- Klavye tuzağı, görünmeyen odak, isimsiz kaldırma/ikon kontrolü.
- Taslak kaybı, yanlış Project/sürüm, örtük başlatma veya dış işlem yetkisi.
- Gerçek çıktı oluşmadan başarı, doğrulanmamış progress veya uydurma ajan faaliyeti.

**QA-05 — Doğrulamayı katmanlandır.** Davranış testleri yanlış komut, veri
kaybı ve yetki ihlalini; geometri testleri sınır/örtüşmeyi; tema/kontrast
ölçümleri okunabilirliği; native yardımcı teknoloji testleri semantiği; görev
oturumları anlaşılabilirliği denetler. Bir katmanın geçmesi diğerini kanıtlamaz.
İnceleme raporu incelenmeyen platformları ve gerçek kullanıcı kanıtı olup
olmadığını açıkça yazar. "Bütün testler yeşil" tek başına UX kabulü değildir.

## 11. Sonraki uygulama sırası

Bu liste, sonraki onaylı görev kartının girdisidir; uygulanmış değişiklik listesi
veya mimari göç kararı değildir.

| Öncelik | Sınırları belirli iş | Kabul kanıtı |
|---|---|---|
| P0 | Composer yerleşimi; kaldırılabilir ek/çıktı chip'leri; görünür çakışma çözümü; açık mod/ana eylem; yerel taslağı genel busy kilidinden ayırma | İlk istek, 12 ek, revizyon çakışması ve bağlantı hatası akışlarında taslak/Project/receipt korunumu; minimum pencere ve klavye incelemesi |
| P1 | Tek takım planı; Hazırla/Başlat aşamaları; sonuç önizleme ve revizyonun aynı sohbet döngüsü; uyarlanan yan panel | İstekten gerçek sonuca ve yeni sürüme yönlendirmesiz görev; dar/geniş geçişte doğru bağlam |
| P2 | Brand ve diğer gelişmiş formlarda içerik/footer düzeni; filtre/menü/badge rolleri; yoğunluk ve boşluk uyumu | Gerçek uzun içerik, kaydet/hata/dirty durumları; tema/ölçek/klavye matrisi |

Kritik yol düzeltilmeden yeni özellik kontrollerini composer'a ekleme. Mevcut
bileşeni gerçek kullanımda doğrula; yalnız ihtiyaç olduğunda ortaklaştır.

## 12. Kaynak dizini

Yukarıdaki bağlantılar ilgili iddianın yanında tutulur. Yeniden araştırmada
başvurulacak birincil kaynaklar:

- W3C: [WCAG 2.2](https://www.w3.org/TR/WCAG22/),
  [WCAG2ICT](https://www.w3.org/TR/wcag2ict/) ve
  [web dışı ICT açıklaması](https://www.w3.org/WAI/standards-guidelines/wcag/non-web-ict/).
- ISO resmi özetleri: [9241-210](https://www.iso.org/standard/77520.html),
  [9241-110](https://www.iso.org/standard/75258.html). Tam metin uygunluk incelemesi yapılmadı.
- Apple: [Buttons](https://developer.apple.com/design/human-interface-guidelines/buttons),
  [Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars),
  [Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts).
- Microsoft: [Fluent buttons](https://fluent2.microsoft.design/components/web/react/core/button/usage),
  [Fluent fields](https://fluent2.microsoft.design/components/web/react/core/field/usage),
  [Human-AI interaction](https://www.microsoft.com/en-us/research/?p=564561).
- Google: [Material Web chips](https://material-web.dev/components/chip/),
  [Compose chips](https://developer.android.com/develop/ui/compose/components/chip).
- Nielsen Norman Group: [Progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/),
  [Usability heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/).
- egui: [Resmi depo](https://github.com/emilk/egui),
  [projeyle eşleşen 0.36.2 Ui API](https://docs.rs/egui/0.36.2/egui/struct.Ui.html).

Kaynaklar zaman içinde değişebilir. Sürümle ilişkili API iddialarında proje
sürümü, platform rehberlerinde ilgili bağlam, standartlarda normatif/bilgilendirici
ayrımı yeniden kontrol edilir. Bu araştırma bir kullanılabilirlik veya
erişilebilirlik sertifikasyonu değildir.
