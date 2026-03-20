using System;
using System.Diagnostics;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new ControllerForm());
    }
}

internal sealed class ControllerForm : Form
{
    private readonly TrackBar _opacitySlider;
    private readonly Label _opacityValue;
    private readonly Label _status;
    private readonly CheckBox _alwaysOnTop;
    private readonly CheckBox _autoWatch;
    private readonly CheckBox _liveApply;
    private readonly CheckBox _clickThrough;
    private readonly Timer _watchTimer;
    private readonly ControllerSettings _settings;
    private readonly Button _applyButton;
    private readonly Button _resetButton;
    private readonly Button _restoreInputButton;
    private readonly Button _openCodexButton;
    private readonly Button _closeButton;

    public ControllerForm()
    {
        Font = new Font("Microsoft YaHei UI", 9F, FontStyle.Regular, GraphicsUnit.Point);
        Text = "Codex 窗口控制器";
        StartPosition = FormStartPosition.CenterScreen;
        ClientSize = new Size(430, 430);
        MinimumSize = new Size(446, 469);
        MaximumSize = new Size(446, 469);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = true;
        ShowInTaskbar = true;
        BackColor = Color.FromArgb(245, 247, 250);
        ForeColor = Color.FromArgb(26, 32, 44);

        _settings = ControllerSettings.Load();

        var headerPanel = new Panel
        {
            BackColor = Color.FromArgb(32, 40, 56),
            Location = new Point(0, 0),
            Size = new Size(ClientSize.Width, 92),
            Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right
        };
        Controls.Add(headerPanel);

        var title = new Label
        {
            Text = "Codex 透明控制器",
            Font = new Font("Microsoft YaHei UI", 15F, FontStyle.Bold),
            ForeColor = Color.White,
            Location = new Point(20, 16),
            Size = new Size(280, 30),
            BackColor = Color.Transparent
        };
        headerPanel.Controls.Add(title);

        var subtitle = new Label
        {
            Text = "动态调透明度、置顶、穿透。关闭控制器时自动恢复 Codex 原样。",
            ForeColor = Color.FromArgb(210, 218, 230),
            Location = new Point(20, 50),
            Size = new Size(370, 24),
            BackColor = Color.Transparent
        };
        headerPanel.Controls.Add(subtitle);

        var surface = new Panel
        {
            BackColor = Color.White,
            Location = new Point(16, 108),
            Size = new Size(398, 236),
            BorderStyle = BorderStyle.FixedSingle
        };
        Controls.Add(surface);

        var opacityTitle = new Label
        {
            Text = "透明度",
            Font = new Font("Microsoft YaHei UI", 10.5F, FontStyle.Bold),
            Location = new Point(16, 16),
            Size = new Size(120, 22)
        };
        surface.Controls.Add(opacityTitle);

        var opacityHint = new Label
        {
            Text = "数值越小越透明，推荐 35% - 60%",
            ForeColor = Color.FromArgb(98, 108, 122),
            Location = new Point(16, 40),
            Size = new Size(260, 20)
        };
        surface.Controls.Add(opacityHint);

        _opacityValue = new Label
        {
            Text = _settings.OpacityPercent.ToString(CultureInfo.InvariantCulture) + "%",
            Font = new Font("Segoe UI", 11F, FontStyle.Bold),
            TextAlign = ContentAlignment.MiddleRight,
            ForeColor = Color.FromArgb(32, 40, 56),
            Location = new Point(312, 16),
            Size = new Size(64, 26)
        };
        surface.Controls.Add(_opacityValue);

        _opacitySlider = new TrackBar
        {
            Minimum = 15,
            Maximum = 100,
            TickFrequency = 5,
            SmallChange = 1,
            LargeChange = 5,
            Value = _settings.OpacityPercent,
            Location = new Point(12, 68),
            Size = new Size(370, 45)
        };
        _opacitySlider.ValueChanged += (_, _) =>
        {
            _opacityValue.Text = _opacitySlider.Value.ToString(CultureInfo.InvariantCulture) + "%";
            SaveSettings();
            if (_liveApply.Checked)
            {
                ApplyCurrentSettings();
            }
        };
        surface.Controls.Add(_opacitySlider);

        _alwaysOnTop = NewCheckBox("让 Codex 始终置顶", 20, 112, _settings.AlwaysOnTop);
        _alwaysOnTop.CheckedChanged += (_, _) =>
        {
            SaveSettings();
            if (_liveApply.Checked)
            {
                ApplyCurrentSettings();
            }
        };
        surface.Controls.Add(_alwaysOnTop);

        _autoWatch = NewCheckBox("自动监听 Codex 窗口", 20, 140, _settings.AutoWatch);
        _autoWatch.CheckedChanged += (_, _) =>
        {
            _watchTimer.Enabled = _autoWatch.Checked;
            SaveSettings();
        };
        surface.Controls.Add(_autoWatch);

        _liveApply = NewCheckBox("拖动滑杆时立即生效", 20, 168, _settings.LiveApply);
        _liveApply.CheckedChanged += (_, _) => SaveSettings();
        surface.Controls.Add(_liveApply);

        _clickThrough = NewCheckBox("让 Codex 鼠标穿透", 20, 196, _settings.ClickThrough);
        _clickThrough.CheckedChanged += (_, _) =>
        {
            SaveSettings();
            if (_liveApply.Checked)
            {
                ApplyCurrentSettings();
            }
        };
        surface.Controls.Add(_clickThrough);

        _status = new Label
        {
            Text = "正在等待 Codex 桌面窗口...",
            Location = new Point(16, 356),
            Size = new Size(398, 22),
            ForeColor = Color.FromArgb(102, 112, 130)
        };
        Controls.Add(_status);

        _applyButton = NewButton("立即应用", 16, 386, 92, 32, true);
        _applyButton.Click += (_, _) => ApplyCurrentSettings();
        Controls.Add(_applyButton);

        _restoreInputButton = NewButton("恢复输入", 116, 386, 92, 32, false);
        _restoreInputButton.Click += (_, _) =>
        {
            _clickThrough.Checked = false;
            ApplyCurrentSettings();
        };
        Controls.Add(_restoreInputButton);

        _resetButton = NewButton("恢复默认", 216, 386, 92, 32, false);
        _resetButton.Click += (_, _) => ResetWindow();
        Controls.Add(_resetButton);

        _openCodexButton = NewButton("打开 Codex", 316, 386, 98, 32, false);
        _openCodexButton.Click += (_, _) => OpenCodex();
        Controls.Add(_openCodexButton);

        _closeButton = NewButton("关闭控制器", 316, 16, 98, 30, false);
        _closeButton.BackColor = Color.FromArgb(53, 62, 81);
        _closeButton.ForeColor = Color.White;
        _closeButton.FlatAppearance.BorderColor = Color.FromArgb(82, 96, 120);
        _closeButton.Click += (_, _) => Close();
        headerPanel.Controls.Add(_closeButton);

        _watchTimer = new Timer { Interval = 1200, Enabled = _settings.AutoWatch };
        _watchTimer.Tick += (_, _) => ApplyCurrentSettings();

        FormClosing += (_, _) =>
        {
            SaveSettings();
            ResetCurrentCodexOnExit();
        };

        ApplyCurrentSettings();
    }

    private CheckBox NewCheckBox(string text, int x, int y, bool isChecked)
    {
        return new CheckBox
        {
            Text = text,
            Checked = isChecked,
            Location = new Point(x, y),
            Size = new Size(250, 22),
            ForeColor = Color.FromArgb(40, 48, 60)
        };
    }

    private Button NewButton(string text, int x, int y, int width, int height, bool primary)
    {
        var button = new Button
        {
            Text = text,
            Location = new Point(x, y),
            Size = new Size(width, height),
            FlatStyle = FlatStyle.Flat,
            Cursor = Cursors.Hand,
            BackColor = primary ? Color.FromArgb(47, 109, 245) : Color.White,
            ForeColor = primary ? Color.White : Color.FromArgb(30, 38, 50)
        };

        button.FlatAppearance.BorderSize = 1;
        button.FlatAppearance.BorderColor = primary
            ? Color.FromArgb(47, 109, 245)
            : Color.FromArgb(210, 218, 230);
        return button;
    }

    private void ApplyCurrentSettings()
    {
        var process = CodexWindow.GetVisibleProcess();
        if (process == null)
        {
            SetStatus("没有找到可见的 Codex 桌面窗口。", false);
            return;
        }

        try
        {
            CodexWindow.Apply(process, _opacitySlider.Value, _alwaysOnTop.Checked, _clickThrough.Checked);
            SetStatus($"已连接 Codex 窗口，进程 PID: {process.Id}", true);
        }
        catch (Exception ex)
        {
            SetStatus(ex.Message, false);
        }
    }

    private void ResetWindow()
    {
        var process = CodexWindow.GetVisibleProcess();
        if (process == null)
        {
            SetStatus("没有找到可见的 Codex 桌面窗口。", false);
            return;
        }

        try
        {
            CodexWindow.Reset(process);
            _opacitySlider.Value = 100;
            _alwaysOnTop.Checked = false;
            _clickThrough.Checked = false;
            SetStatus("Codex 窗口已恢复默认状态。", true);
            SaveSettings();
        }
        catch (Exception ex)
        {
            SetStatus(ex.Message, false);
        }
    }

    private void OpenCodex()
    {
        try
        {
            Process.Start("explorer.exe", "shell:AppsFolder\\OpenAI.Codex_2p2nqsd0c76g0!Codex");
            System.Threading.Thread.Sleep(900);
            ApplyCurrentSettings();
        }
        catch
        {
            SetStatus("启动 Codex 桌面版失败。", false);
        }
    }

    private void SaveSettings()
    {
        _settings.OpacityPercent = _opacitySlider.Value;
        _settings.AlwaysOnTop = _alwaysOnTop.Checked;
        _settings.AutoWatch = _autoWatch.Checked;
        _settings.LiveApply = _liveApply.Checked;
        _settings.ClickThrough = _clickThrough.Checked;
        _settings.Save();
    }

    private void ResetCurrentCodexOnExit()
    {
        var process = CodexWindow.GetVisibleProcess();
        if (process == null)
        {
            return;
        }

        try
        {
            CodexWindow.Reset(process);
        }
        catch
        {
            // Closing should stay silent.
        }
    }

    private void SetStatus(string text, bool success)
    {
        _status.Text = text;
        _status.ForeColor = success
            ? Color.FromArgb(38, 110, 70)
            : Color.FromArgb(196, 69, 69);
    }
}

internal sealed class ControllerSettings
{
    private static readonly string SettingsPath = Path.Combine(
        AppDomain.CurrentDomain.BaseDirectory,
        "codex-window-controller.settings.txt");

    public int OpacityPercent { get; set; } = 45;
    public bool AlwaysOnTop { get; set; } = true;
    public bool AutoWatch { get; set; } = true;
    public bool LiveApply { get; set; } = true;
    public bool ClickThrough { get; set; }

    public static ControllerSettings Load()
    {
        var settings = new ControllerSettings();
        if (!File.Exists(SettingsPath))
        {
            return settings;
        }

        foreach (var line in File.ReadAllLines(SettingsPath))
        {
            var parts = line.Split(new[] { '=' }, 2);
            if (parts.Length != 2)
            {
                continue;
            }

            switch (parts[0])
            {
                case "OpacityPercent":
                    if (int.TryParse(parts[1], out var opacity))
                    {
                        settings.OpacityPercent = Math.Max(15, Math.Min(100, opacity));
                    }
                    break;
                case "AlwaysOnTop":
                    settings.AlwaysOnTop = ParseBool(parts[1], settings.AlwaysOnTop);
                    break;
                case "AutoWatch":
                    settings.AutoWatch = ParseBool(parts[1], settings.AutoWatch);
                    break;
                case "LiveApply":
                    settings.LiveApply = ParseBool(parts[1], settings.LiveApply);
                    break;
                case "ClickThrough":
                    settings.ClickThrough = ParseBool(parts[1], settings.ClickThrough);
                    break;
            }
        }

        return settings;
    }

    public void Save()
    {
        File.WriteAllLines(SettingsPath, new[]
        {
            "OpacityPercent=" + OpacityPercent.ToString(CultureInfo.InvariantCulture),
            "AlwaysOnTop=" + AlwaysOnTop,
            "AutoWatch=" + AutoWatch,
            "LiveApply=" + LiveApply,
            "ClickThrough=" + ClickThrough
        });
    }

    private static bool ParseBool(string value, bool fallback)
    {
        return bool.TryParse(value, out var parsed) ? parsed : fallback;
    }
}

internal static class CodexWindow
{
    public static Process GetVisibleProcess()
    {
        var candidates = Process.GetProcessesByName("Codex");
        Array.Sort(candidates, (left, right) => right.Id.CompareTo(left.Id));

        foreach (var process in candidates)
        {
            if (process.MainWindowHandle != IntPtr.Zero)
            {
                return process;
            }
        }

        return null;
    }

    public static void Apply(Process process, int opacityPercent, bool alwaysOnTop, bool clickThrough)
    {
        var handle = process.MainWindowHandle;
        if (handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("Codex 窗口句柄不可用。");
        }

        var style = NativeMethods.GetWindowLong(handle, NativeMethods.GwlExStyle);
        var newStyle = style | NativeMethods.WsExLayered;
        if (clickThrough)
        {
            newStyle |= NativeMethods.WsExTransparent;
        }
        else
        {
            newStyle &= ~NativeMethods.WsExTransparent;
        }

        NativeMethods.SetWindowLong(handle, NativeMethods.GwlExStyle, newStyle);

        var alpha = (byte)Math.Round(opacityPercent / 100.0 * 255);
        if (!NativeMethods.SetLayeredWindowAttributes(handle, 0, alpha, NativeMethods.LwaAlpha))
        {
            throw new InvalidOperationException("设置 Codex 窗口透明度失败。");
        }

        var topMode = alwaysOnTop ? NativeMethods.HwndTopMost : NativeMethods.HwndNoTopMost;
        if (!NativeMethods.SetWindowPos(
                handle,
                topMode,
                0,
                0,
                0,
                0,
                NativeMethods.SwpNoMove | NativeMethods.SwpNoSize | NativeMethods.SwpNoActivate | NativeMethods.SwpShowWindow))
        {
            throw new InvalidOperationException("设置 Codex 置顶状态失败。");
        }
    }

    public static void Reset(Process process)
    {
        var handle = process.MainWindowHandle;
        if (handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("Codex 窗口句柄不可用。");
        }

        var style = NativeMethods.GetWindowLong(handle, NativeMethods.GwlExStyle);
        var newStyle = (style | NativeMethods.WsExLayered) & ~NativeMethods.WsExTransparent;
        NativeMethods.SetWindowLong(handle, NativeMethods.GwlExStyle, newStyle);
        NativeMethods.SetLayeredWindowAttributes(handle, 0, 255, NativeMethods.LwaAlpha);
        NativeMethods.SetWindowPos(
            handle,
            NativeMethods.HwndNoTopMost,
            0,
            0,
            0,
            0,
            NativeMethods.SwpNoMove | NativeMethods.SwpNoSize | NativeMethods.SwpNoActivate | NativeMethods.SwpShowWindow);
    }
}

internal static class NativeMethods
{
    public const int GwlExStyle = -20;
    public const int WsExLayered = 0x00080000;
    public const int WsExTransparent = 0x00000020;
    public const uint LwaAlpha = 0x2;
    public static readonly IntPtr HwndTopMost = new IntPtr(-1);
    public static readonly IntPtr HwndNoTopMost = new IntPtr(-2);
    public const uint SwpNoSize = 0x0001;
    public const uint SwpNoMove = 0x0002;
    public const uint SwpNoActivate = 0x0010;
    public const uint SwpShowWindow = 0x0040;

    [DllImport("user32.dll", SetLastError = true)]
    public static extern int GetWindowLong(IntPtr hWnd, int nIndex);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern int SetWindowLong(IntPtr hWnd, int nIndex, int dwNewLong);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetLayeredWindowAttributes(IntPtr hwnd, uint crKey, byte bAlpha, uint dwFlags);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(
        IntPtr hWnd,
        IntPtr hWndInsertAfter,
        int x,
        int y,
        int cx,
        int cy,
        uint flags);
}
