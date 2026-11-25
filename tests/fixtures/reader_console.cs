// Controlled CI fixture: no packages, UI, network or writes to the selected file.
using System;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

internal static class Program
{
    private const int Limit = 65536;
    private sealed record Preview(string kind, string content, bool truncated);

    private static Preview Describe(byte[] bytes, bool truncated)
    {
        try
        {
            string text = new UTF8Encoding(false, true).GetString(bytes);
            if (!text.Contains('\0')) return new Preview("text", text, truncated);
        }
        catch (DecoderFallbackException) { }
        return new Preview("hex", Convert.ToHexString(bytes).ToLowerInvariant(), truncated);
    }

    private static Preview ReadDocument(string path, bool nativeArgument)
    {
        byte[] buffer = new byte[Limit + 1];
        int length = 0;
        byte[]? raw = nativeArgument && !OperatingSystem.IsWindows() ? RawLastArgument() : null;
        bool needsRaw = false;
        if (raw != null)
        {
            try { _ = new UTF8Encoding(false, true).GetString(raw); }
            catch (DecoderFallbackException) { needsRaw = true; }
        }
        if (needsRaw)
        {
            byte[] terminated = new byte[raw!.Length + 1];
            Buffer.BlockCopy(raw, 0, terminated, 0, raw.Length);
            int descriptor = NativeOpen(terminated, 0);
            if (descriptor < 0) throw new IOException("Cannot open the native filename");
            try
            {
                while (length < buffer.Length)
                {
                    byte[] chunk = new byte[buffer.Length - length];
                    nint count = NativeRead(descriptor, chunk, (nuint)chunk.Length);
                    if (count < 0) throw new IOException("Cannot read the native filename");
                    if (count == 0) break;
                    Buffer.BlockCopy(chunk, 0, buffer, length, (int)count);
                    length += (int)count;
                }
            }
            finally { NativeClose(descriptor); }
        }
        else
        {
            using var input = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
            while (length < buffer.Length)
            {
                int count = input.Read(buffer, length, buffer.Length - length);
                if (count == 0) break;
                length += count;
            }
        }
        byte[] bytes = buffer.Take(Math.Min(length, Limit)).ToArray();
        return Describe(bytes, length > Limit);
    }

    // .NET strings cannot represent invalid UTF-8 Unix names. This fixture keeps
    // their native argv bytes so the CLI's OsStr transport is tested end to end.
    private static byte[]? RawLastArgument()
    {
        if (OperatingSystem.IsLinux())
        {
            byte[] command = File.ReadAllBytes("/proc/self/cmdline");
            int end = command.Length;
            while (end > 0 && command[end - 1] == 0) end--;
            int begin = end;
            while (begin > 0 && command[begin - 1] != 0) begin--;
            return command[begin..end];
        }
        if (OperatingSystem.IsMacOS())
        {
            IntPtr argv = Marshal.ReadIntPtr(NativeArgv());
            IntPtr last = IntPtr.Zero;
            for (int i = 0; i < 256; i++)
            {
                IntPtr next = Marshal.ReadIntPtr(argv, i * IntPtr.Size);
                if (next == IntPtr.Zero) break;
                last = next;
            }
            if (last == IntPtr.Zero) return null;
            int count = 0;
            while (count < 65536 && Marshal.ReadByte(last, count) != 0) count++;
            byte[] bytes = new byte[count];
            Marshal.Copy(last, bytes, 0, count);
            return bytes;
        }
        return null;
    }

    [DllImport("libc", EntryPoint = "open", SetLastError = true)]
    private static extern int NativeOpen(byte[] path, int flags);
    [DllImport("libc", EntryPoint = "read", SetLastError = true)]
    private static extern nint NativeRead(int descriptor, byte[] buffer, nuint size);
    [DllImport("libc", EntryPoint = "close")]
    private static extern int NativeClose(int descriptor);
    [DllImport("libc", EntryPoint = "_NSGetArgv")]
    private static extern IntPtr NativeArgv();

    private static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        try
        {
            if (args.Length == 1 && args[0] == "--crexe-self-test")
            {
                return Describe(Encoding.UTF8.GetBytes("reader é"), false).content == "reader é"
                    && Describe(new byte[] { 0, 255, 65, 128 }, false).content == "00ff4180"
                    && Describe(Array.Empty<byte>(), false).content == "" ? 0 : 1;
            }
            if (args.Length == 2 && args[0] == "--crexe-reader-check")
            {
                Console.WriteLine(JsonSerializer.Serialize(ReadDocument(args[1], false)));
                return 0;
            }
            if (args.Length == 0)
            {
                Console.WriteLine("RECIPE_LEGACY_NO_DOCUMENT");
                return 0;
            }
            if (args.Length != 1) throw new ArgumentException("Expected one document argument");
            Console.WriteLine(JsonSerializer.Serialize(ReadDocument(args[0], true)));
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine("Reader fixture could not open the document: " + error.Message);
            return 2;
        }
    }
}
