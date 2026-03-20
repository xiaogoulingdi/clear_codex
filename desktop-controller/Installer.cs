using System;
using System.IO;
using System.Reflection;
using System.Windows.Forms;

internal static class InstallerProgram
{
    [STAThread]
    private static void Main()
    {
        try
        {
            var baseDir = Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location) ?? AppDomain.CurrentDomain.BaseDirectory;
            var sourceExe = Path.Combine(baseDir, "CodexWindowController.exe");
            var installDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "CodexWindowController");
            var targetExe = Path.Combine(installDir, "CodexWindowController.exe");
            var desktopShortcut = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
                "Codex 窗口控制器.lnk");
            var startMenuDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.Programs),
                "Codex Window Controller");
            var startMenuShortcut = Path.Combine(startMenuDir, "Codex 窗口控制器.lnk");

            if (!File.Exists(sourceExe))
            {
                MessageBox.Show(
                    "未找到 CodexWindowController.exe，请确保安装器和主程序放在同一目录。",
                    "安装失败",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
                return;
            }

            Directory.CreateDirectory(installDir);
            Directory.CreateDirectory(startMenuDir);
            File.Copy(sourceExe, targetExe, true);

            CreateShortcut(desktopShortcut, targetExe, installDir, "Codex 桌面版窗口动态控制器");
            CreateShortcut(startMenuShortcut, targetExe, installDir, "Codex 桌面版窗口动态控制器");

            MessageBox.Show(
                "安装完成。\n\n桌面和开始菜单快捷方式都已创建。",
                "Codex 窗口控制器",
                MessageBoxButtons.OK,
                MessageBoxIcon.Information);
        }
        catch (Exception ex)
        {
            MessageBox.Show(
                "安装失败：\n" + ex.Message,
                "Codex 窗口控制器",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
        }
    }

    private static void CreateShortcut(string shortcutPath, string targetPath, string workingDirectory, string description)
    {
        var shellType = Type.GetTypeFromProgID("WScript.Shell");
        if (shellType == null)
        {
            throw new InvalidOperationException("无法创建快捷方式，系统缺少 WScript.Shell。");
        }

        var shell = Activator.CreateInstance(shellType);
        if (shell == null)
        {
            throw new InvalidOperationException("无法初始化快捷方式组件。");
        }

        dynamic shortcut = shellType.InvokeMember(
            "CreateShortcut",
            BindingFlags.InvokeMethod,
            null,
            shell,
            new object[] { shortcutPath });

        shortcut.TargetPath = targetPath;
        shortcut.WorkingDirectory = workingDirectory;
        shortcut.Description = description;
        shortcut.Save();
    }
}
