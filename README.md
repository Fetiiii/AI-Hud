# AI HUD

Linux (KDE/CachyOS) için Claude Code ve Codex kullanım limitlerini gösteren,
sistem tepsisinden açılan hafif bir HUD. Backend Rust (Tauri), arayüz Svelte.

## Ne gösteriyor

- Claude Code: 5 saatlik ve haftalık kullanım yüzdesi + sıfırlanma zamanı
- Codex: aynı metrikler (5 saatlik + haftalık, `rate_limit.primary_window` / `secondary_window`)
- Aktif oturumun context penceresi doluluk oranı - hem Claude Code hem Codex
  için, tamamen yerel transcript/rollout dosyalarından, ağ çağrısı yok

Sistem tepsisindeki simgeye tıklamak küçük bir popover açar; popover içindeki
herhangi bir metriğe çift tıklamak (veya popover'daki genişlet butonu) daha
detaylı bir pencere açar.

## Veri kaynağı

- Claude Code: `~/.claude/.credentials.json`'daki OAuth token'ıyla,
  dokümante edilmemiş ama ücretsiz (inference maliyeti olmayan)
  `GET https://api.anthropic.com/api/oauth/usage` uç noktası. Bu, `claude`
  CLI'nin `/usage` komutunun kullandığı veriyle aynı.
- Codex: iki kaynak birden. `~/.codex/sessions/**/rollout-*.jsonl` içindeki
  `rate_limits` bloğu (kullanım yüzdesi, pencere süresi, plan, kredi) ağ
  gerektirmez ve token süresi dolmaz; `GET
  https://chatgpt.com/backend-api/wham/usage` ise daha tazedir. Hangisi daha
  yeniyse o kullanılır.
- Pencere adları sabit değil, süreden türetilir: 300 dk → "5 saatlik",
  10080 → "Haftalık", 43200 → "Aylık". Ücretsiz ChatGPT planında tek pencere
  vardır ve o aylıktır - sabit "5 saatlik" varsaymak yanlış olur.
- Oturum maliyeti: <https://models.dev/api.json> (anahtarsız, günlük
  önbellek). `~/.config/ai-hud/prices.json` yazarsan senin değerlerin ezer.
  Rakam API liste fiyatına göre tahmindir; abonelikte tahsil edilmez.
- Context kullanımı: Claude Code için `~/.claude/projects/**/*.jsonl`, Codex
  için `~/.codex/sessions/**/rollout-*.jsonl` (`token_count` olaylarındaki
  `last_token_usage` + `model_context_window`). Her ikisi de `notify` ile
  dosya değişikliklerinde canlı izleniyor.

Bu uç noktalar resmi/dokümante değil - Anthropic/OpenAI önceden haber
vermeden değiştirebilir. Bir sağlayıcı başarısız olursa HUD o kartı hatayla
birlikte gösterir, çökmez.

## Kurulum

CachyOS/Arch'ta önce Tauri'nin sistem bağımlılıklarını ve pnpm'i kur:

```bash
sudo pacman -S --needed pnpm webkit2gtk-4.1 base-devel curl wget file \
  openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

Sonra derle ve kur:

```bash
pnpm install
pnpm tauri build --no-bundle   # Arch'ta deb/rpm/AppImage araçları yok
./install.sh                    # ~/.local/bin + uygulama menüsü girdisi
./install.sh --autostart        # oturum açılışında da başlasın
```

Artık uygulama menüsünden "AI HUD" olarak açılıyor, ya da terminalden
`ai-hud`. Kaldırmak için `./install.sh --uninstall`.

## Kullanım

Açılışta HUD ekrana gelir. Gövdesine:

- **tek tık** → küçülür (iki sağlayıcının yüzdesi kalır)
- **çift tık** → detaya büyür (tüm pencereler, oturum dökümü, maliyet)

Üstteki şeritten sürüklenir. Tepsi simgesine sağ tıklayınca gizle/göster ve
çıkış menüsü gelir — Linux'ta tepsi arka uçları sol tık olayı üretmediği için
menü tek giriş noktasıdır.

## Geliştirme

```bash
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
