# Stream Deck plugin for Roland V-60HD

Elgato Stream DeckからRoland V-60HDをLAN（TCP 8023）で操作するプラグインです。

このプロジェクトはRoland非公式です。

公式ドキュメント: [V-60HD Reference Manual (LAN / RS-232)](https://static.roland.com/assets/media/pdf/V-60HD_reference_v31_eng04_W.pdf)

## Download

[最新リリース](https://github.com/MikanseiLaboratory/streamdeck-roland-v60hd/releases/latest)から`.streamDeckPlugin`をダウンロードし、ダブルクリックしてStream Deckにインストールしてください。

## 動作

- Select PGM / Select PRV は Tally Check でキーを点灯できます（Off / PRV / PGM / PRV/PGM）。ランプは `TLY;` を約 500ms 間隔でポーリングして取得します（Red=PGM、Green=PST）。本体は CUT 後も TLY/QPL を自発送信しません。
- Select AUX は `QPL:7` の AUX フィールド、Composition の PinP 1/2・SPLIT・DSK・OUTPUT FADE は同スナップショットで点灯します。DSK PVW と AUTO MIXING は QPL に対応ビットがないため点灯しません。
- CUT / AUTO は ACK 後に TLY と QPL を取り直し、PGM/PRV/AUX/Composition キーへ反映します。
- 同じ IP のキーはTCP接続を1本だけ共有します。V-60HD は同時に **1 接続** しか受け付けないため、V-60HD RCS や他の Telnet は閉じてください。
- パスワードはありません。コマンドは STX (0x02) 付きで、ACK を待ってから次を送ります。
- `ACS` はファームウェア 3.02 の LAN では応答しないため、プラグインからは送りません。
- 切断時は1s → 2s → 4s …（上限30s）で再接続します。
- キーが画面から消えてもすぐには切断せず、約30秒のアイドル後に接続を閉じます。
