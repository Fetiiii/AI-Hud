# AI HUD

Linux (KDE/CachyOS) için Claude Code ve Codex kullanım limitlerini gösteren,
sistem tepsisinden açılan hafif bir HUD. Backend Rust (Tauri), arayüz Svelte.

## Ne gösteriyor

- Claude Code: 5 saatlik ve haftalık kullanım yüzdesi + sıfırlanma zamanı
- Codex: aynı metrikler (5 saatlik + haftalık, `rate_limit.primary_window` / `secondary_window`)
- Aktif oturumun context penceresi doluluk oranı - şu an sadece Claude Code için
  (yerel transcript dosyalarından, ağ çağrısı yok); Codex'in yerel oturum
  loglarının formatı henüz doğrulanmadığı için Codex context ölçümü boş döner,
  bkz. `src-tauri/src/context.rs`

Sistem tepsisindeki simgeye tıklamak küçük bir popover açar; popover içindeki
herhangi bir metriğe çift tıklamak (veya popover'daki genişlet butonu) daha
detaylı bir pencere açar.

## Veri kaynağı

- Claude Code: `~/.claude/.credentials.json`'daki OAuth token'ıyla,
  dokümante edilmemiş ama ücretsiz (inference maliyeti olmayan)
  `GET https://api.anthropic.com/api/oauth/usage` uç noktası. Bu, `claude`
  CLI'nin `/usage` komutunun kullandığı veriyle aynı.
- Context kullanımı: `~/.claude/projects/**/*.jsonl` transkript dosyaları,
  dosya değişikliklerinde `notify` ile canlı izleniyor.
- Codex: `~/.codex/auth.json`'daki `tokens.access_token` / `tokens.account_id`
  ile `GET https://chatgpt.com/backend-api/wham/usage`. Şema, steipete/CodexBar
  (aynı işi macOS'ta yapan açık kaynak menu-bar uygulaması) kodundan
  doğrulandı; gerçek bir hesapla uçtan uca henüz test edilmedi.

Bu uç noktalar resmi/dokümante değil - Anthropic/OpenAI önceden haber
vermeden değiştirebilir. Bir sağlayıcı başarısız olursa HUD o kartı hatayla
birlikte gösterir, çökmez.

## Geliştirme

CachyOS/Arch'ta önce Tauri'nin sistem bağımlılıklarını kur:

```bash
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

Sonra:

```bash
pnpm install
pnpm tauri dev
```

Sadece Rust tarafını derleme kontrolü için:

```bash
cd src-tauri && cargo check
```

Sadece frontend tip kontrolü için:

```bash
pnpm check
```
