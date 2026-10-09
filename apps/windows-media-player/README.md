# Windows Media Player — проєкт (робоча назва: ZillaPlayer)

> **Статус: P1 / local playback alpha (код додано).** Є нативний інтерфейс Rust + Slint, а також реалізовані Open File, Play/Pause, Stop, Seek та гучність через фоновий аудіоворкер Rodio/CPAL. **Компіляцію й відтворення на Windows ще не перевірено**. EQ/візуалізатор поки декоративні; downloader не реалізований. Назва продукту попередня.

Ця папка ізольована в гілці `apps/windows-media-player`. Гілка `main` залишається універсальним Appbootstrap. Ми свідомо **не** переносимо специфічні правила плеєра у головний шаблон.

## Ідея продукту

Легкий нативний музичний плеєр для Windows 10/11 (x64, пізніше ARM64) у дусі AIMP/Winamp: локальні аудіофайли, потужний еквалайзер та DSP, візуалізації, швидкий каталог і плейлисти, модульність, а також **окремий** менеджер завантажень на базі yt-dlp з підтримкою допустимих джерел та власних/дозволених користувачеві медіа.

**Не робимо одразу:** браузер, відеоредактор, системний еквалайзер для всіх програм, DRM-обхід, власні екстрактори тисяч сайтів, VST-хост, мобільний застосунок чи хмарний сервіс.

## Ключове технічне рішення

- Основна мова: **Rust** (кандидат, приймаємо для прототипу).
- GUI: **Slint** (кандидат; пройти технічний + ліцензійний spike).
- Audio output (P1 alpha): **Rodio 0.21.1 → CPAL** → WASAPI shared mode на Windows. Далі дослідимо прямий CPAL із власним DSP callback.
- Decode: **Symphonia через Rodio**, MP3/FLAC/WAV/OGG/M4A (фактичні файли ще потрібно протестувати).
- DSP: власний невеликий real-time-safe audio graph, biquad EQ, gain / limiter; FFT окремим споживачем.
- Library: **SQLite** (WAL/FTS5 за потреби), індексація у фонових потоках.
- Downloads: **yt-dlp зовнішнім процесом**; FFmpeg/ffprobe + актуальні YouTube runtime/EJS вимоги як залежності, що перевіряються.
- Плагіни: власні вбудовані інтерфейси на старті, розширення зовнішніми адаптерами після визначення ABI / безпеки.

Усі залежності й версії треба зафіксувати після proof-of-concept; список тут **не є затвердженим Cargo.lock**.

## Документація
- [Roadmap](ROADMAP.md) — порядок розробки, критерії прийняття, ризики.
- [Специфікація продукту](docs/PRODUCT_SPEC.md).
- [Архітектура](docs/ARCHITECTURE.md).
- [Аудіоядро](docs/AUDIO_ENGINE.md).
- [DSP + візуалізації](docs/DSP_VISUALIZATIONS.md).
- [Менеджер завантажень](docs/DOWNLOADER.md).
- [Плагіни](docs/PLUGIN_SYSTEM.md).
- [Продуктивність і тести](docs/PERFORMANCE_TEST_PLAN.md).
- [Безпека / права / дистрибуція](docs/SECURITY_LEGAL.md).
- [Технологічні рішення](docs/TECH_STACK_RESEARCH.md).

## Запуск UI-прототипу на Windows

Встановіть Rust stable (MSVC), а також Visual Studio Build Tools / Desktop development with C++.

```powershell
cd apps/windows-media-player
cargo run
```

В UI кнопка **Open File** відкриває системний діалог, далі Rust надсилає декодований звук через CPAL/WASAPI; є Play/Pause/Stop/Seek/Volume. **Це alpha-код, а не підтверджений working build.** EQ-візуалізація, плейлист і downloader ще демо. Докладніше: [IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md), [DEVELOPMENT_NOTES.md](docs/DEVELOPMENT_NOTES.md).

## Перший наступний крок

Зробити **Phase 0 / технологічні spikes**, потім робочий vertical slice `Open MP3 → Decode → PCM → DSP bypass → WASAPI → Pause/Seek`. Не починати з downloader або складних скінів: це приховає проблеми аудіоядра.

## Статус перевірки

UI + базовий код аудіовідтворення додано до GitHub; компіляція на Windows і реальний аудіовивід **ще не перевірені**. DSP/FFT, завантажувач та інсталятор **ще не реалізовані**.

