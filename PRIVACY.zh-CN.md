# 隐私说明

[English](PRIVACY.md)

PairDock 是一个本地工具：不需要账号，不收集使用数据，不上报崩溃信息，没有广告。它唯一会连接的是 GitHub，用来检查更新，而且可以关闭。

## 网络

PairDock 联网只有一个用途：到 GitHub 检查是否有新版本。

- **什么时候**：每次启动时检查一次（可在设置里关闭"启动时检查更新"），以及你点击"检查更新"时。
- **请求什么**：文件 `https://github.com/n2ns/pairdock/releases/latest/download/latest.json`，里面列出最新版本号，由 GitHub 的下载服务器提供。请求里不包含任何关于你、你的设备或设置的信息；和所有网络请求一样，对方能看到你的 IP 地址，请求标识为 `tauri-plugin-updater/<版本号>`（更新组件的版本）。
- **下载更新**：只有在你点击"安装并重启"之后才会下载。PairDock 从同一个 GitHub 发布页下载安装程序，并在运行前校验它的签名。

设置页里的"GitHub 上的源代码"链接和"在 GitHub 上点 Star"按钮会在你的浏览器中打开仓库页面，PairDock 本身不发送任何内容。

GitHub 如何处理这些请求，适用 [GitHub 隐私声明](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement)。关闭"启动时检查更新"后，除非你手动点击"检查更新"，PairDock 不会发起任何网络请求。

它的界面随程序一起打包，运行在 Microsoft Edge WebView2 中，PairDock 不会让 WebView2 从互联网加载任何内容。

PairDock 依赖的两个 Windows 组件由微软维护，适用微软自己的条款：

- **WebView2** 通过微软的更新服务自行更新，与 PairDock 无关。
- **安装程序**只在系统缺少 WebView2 运行时的情况下，才会从微软下载它。Windows 11 和大多数保持更新的 Windows 10 已经自带。

## PairDock 会访问什么

- **罗技接收器**：通过 Windows 自带的 HID 驱动访问。PairDock 会读取配对在接收器上的设备的名称、型号、序列号、电量和按键能力，并发送配对、解除配对和设置按键的命令。
- **鼠标按键**：只限于你设置为"快捷键"或"禁用"的那几个键。这些键被按下时，鼠标会把按键事件交给 PairDock，而不是交给 Windows。PairDock 看不到鼠标移动，也看不到其他按键。
- **键盘**：PairDock 从不读取你输入的内容。只有在它自己的窗口里录制快捷键时，它才能看到你按下的键。它只会为你设置的快捷键、在对应的鼠标按键被按住时，向 Windows 发送按键。

## PairDock 会保存什么

所有数据都只保存在你的电脑上：

| 内容 | 位置 |
|---|---|
| 按键设置：设备型号编号和每个按键的动作 | `%APPDATA%\dev.pairdock.desktop\config.json` |
| 语言、外观和检查更新设置 | `%LOCALAPPDATA%\dev.pairdock.desktop`（WebView2 数据） |
| 开机启动项（仅在开启"开机自动启动"时） | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` 下的 `PairDock` 值 |

PairDock 不写日志文件。设备序列号只用于显示，不会保存。

## 协议日志

命令行工具（`pairdock-cli --trace <文件>`）可以记录与接收器之间的全部通信，用于排查问题。只有你主动要求时才会生成这个文件，而且只写到你指定的位置。文件里包含设备地址、序列号，以及配对过程中的确认码，附到问题报告之前请先检查一下。

## 删除你的数据

卸载 PairDock，然后删除上面列出的两个文件夹。卸载前先关闭"开机自动启动"，即可删除开机启动项。

## 变更

如果将来 PairDock 需要因为其他用途联网，我们会在发布该版本之前更新这份说明，并在[更新日志](CHANGELOG.md)中注明。

有疑问请[提交问题](https://github.com/n2ns/pairdock/issues)。
