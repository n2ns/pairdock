# 隐私说明

[English](PRIVACY.md)

PairDock 是一个纯本地工具：不需要账号，不收集使用数据，不上报崩溃信息，没有广告，也不连接互联网。

## 网络

PairDock 不发起任何网络请求。它的界面随程序一起打包，运行在 Microsoft Edge WebView2 中，PairDock 不会让 WebView2 从互联网加载任何内容。

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
| 语言和外观设置 | `%LOCALAPPDATA%\dev.pairdock.desktop`（WebView2 数据） |
| 开机启动项（仅在开启"开机自动启动"时） | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` 下的 `PairDock` 值 |

PairDock 不写日志文件。设备序列号只用于显示，不会保存。

## 协议日志

命令行工具（`pairdock-cli --trace <文件>`）可以记录与接收器之间的全部通信，用于排查问题。只有你主动要求时才会生成这个文件，而且只写到你指定的位置。文件里包含设备地址、序列号，以及配对过程中的确认码，附到问题报告之前请先检查一下。

## 删除你的数据

卸载 PairDock，然后删除上面列出的两个文件夹。卸载前先关闭"开机自动启动"，即可删除开机启动项。

## 变更

如果将来 PairDock 需要联网（例如检查更新），我们会在发布该版本之前更新这份说明，并在[更新日志](CHANGELOG.md)中注明。

有疑问请[提交问题](https://github.com/n2ns/pairdock/issues)。
