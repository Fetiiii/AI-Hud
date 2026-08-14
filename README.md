# AI HUD

Linux (KDE/CachyOS) için Claude Code ve Codex kullanım limitlerini gösteren,
sistem tepsisinden açılan hafif bir HUD. Backend Rust (Tauri), arayüz Svelte.

## Ne gösteriyor

- Claude Code: 5 saatlik ve haftalık kullanım yüzdesi + sıfırlanma zamanı
- Codex: aynı metrikler (uç nokta/şema henüz gerçek bir hesapla doğrulanmadı, bkz. `src-tauri/src/providers/codex.rs`)
- Aktif oturumun context penceresi doluluk oranı (yerel transcript dosyalarından, ağ çağrısı yok)

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
- Codex tarafı benzer mantıkla `~/.codex/auth.json` + reverse-engineer
  edilmiş `/wham/usage` uç noktasını deniyor; bu kısım gerçek bir Codex
  hesabıyla test edilip doğrulanmadı.

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
