# Stream Deck plugin for Roland V-60HD

Elgato Stream DeckからRoland V-60HDをLAN（TCP 8023）で操作するプラグインです。

このプロジェクトはRoland非公式です。

公式ドキュメント: [V-60HD Reference Manual (LAN / RS-232)](https://static.roland.com/assets/media/pdf/V-60HD_reference_v31_eng04_W.pdf)

## Download

[最新リリース](https://github.com/MikanseiLaboratory/streamdeck-roland-v60hd/releases/latest)から`.streamDeckPlugin`をダウンロードし、ダブルクリックしてStream Deckにインストールしてください。

## 動作

- Select PGMとSelect PRVは、Tally Checkでキーをスイッチャーのように点灯できます（Off / PRV / PGM / PRV/PGM）。8チャンネル（SDI 1–4 / HDMI 5 / HDMI-RGB 6 / Still 7–8）が対象です。
- 同じ IP のキーはTCP接続を1本だけ共有します。V-60HD は同時に **1 接続** しか受け付けないため、V-60HD RCS や他の Telnet は閉じてください。
- パスワードはありません。コマンドは STX (0x02) 付きで、ACK を待ってから次を送ります。
- 切断時は1s → 2s → 4s …（上限30s）で再接続します。
- キーが画面から消えてもすぐには切断せず、約30秒のアイドル後に接続を閉じます。
