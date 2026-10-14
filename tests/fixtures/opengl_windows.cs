using System;
using OpenTK.Graphics.OpenGL4;
using OpenTK.Mathematics;
using OpenTK.Windowing.Common;
using OpenTK.Windowing.Desktop;

sealed class GraphicsCheck : GameWindow
{
    int vao, buffer, program;
    public GraphicsCheck() : base(GameWindowSettings.Default, new NativeWindowSettings {
        ClientSize = new Vector2i(320, 240), Title = "CREXE OpenGL fixture",
        APIVersion = new Version(3, 3), Profile = ContextProfile.Core, StartVisible = false
    }) { }
    static int Shader(ShaderType type, string source) {
        int shader = GL.CreateShader(type);
        GL.ShaderSource(shader, source); GL.CompileShader(shader);
        GL.GetShader(shader, ShaderParameter.CompileStatus, out int ok);
        if (ok == 0) throw new Exception(GL.GetShaderInfoLog(shader));
        return shader;
    }
    protected override void OnLoad() {
        base.OnLoad();
        int vertex = Shader(ShaderType.VertexShader, "#version 330 core\nlayout(location=0) in vec3 p; void main(){gl_Position=vec4(p,1.0);}");
        int fragment = Shader(ShaderType.FragmentShader, "#version 330 core\nout vec4 c; void main(){c=vec4(0.2,0.8,0.3,1.0);}");
        program = GL.CreateProgram(); GL.AttachShader(program, vertex); GL.AttachShader(program, fragment); GL.LinkProgram(program);
        GL.GetProgram(program, GetProgramParameterName.LinkStatus, out int ok);
        if (ok == 0) throw new Exception(GL.GetProgramInfoLog(program));
        GL.DeleteShader(vertex); GL.DeleteShader(fragment);
        float[] vertices = {-0.7f,-0.7f,0, 0.7f,-0.7f,0, 0,0.7f,0};
        vao=GL.GenVertexArray(); GL.BindVertexArray(vao);
        buffer=GL.GenBuffer(); GL.BindBuffer(BufferTarget.ArrayBuffer, buffer);
        GL.BufferData(BufferTarget.ArrayBuffer, vertices.Length*sizeof(float), vertices, BufferUsageHint.StaticDraw);
        GL.VertexAttribPointer(0,3,VertexAttribPointerType.Float,false,3*sizeof(float),0); GL.EnableVertexAttribArray(0);
    }
    protected override void OnRenderFrame(FrameEventArgs e) {
        base.OnRenderFrame(e); GL.Viewport(0,0,320,240); GL.Clear(ClearBufferMask.ColorBufferBit);
        GL.UseProgram(program); GL.BindVertexArray(vao); GL.DrawArrays(PrimitiveType.Triangles,0,3); GL.Finish();
        if (GL.GetError()!=ErrorCode.NoError) throw new Exception("OpenGL rendering failed");
        Console.WriteLine("CREXE_GRAPHICS_OK " + GL.GetString(StringName.Version)); SwapBuffers(); Close();
    }
    protected override void OnUnload() { GL.DeleteBuffer(buffer); GL.DeleteVertexArray(vao); GL.DeleteProgram(program); base.OnUnload(); }
}
static class Program {
    public static int Main(string[] args) {
        if (args.Length>0 && args[0]=="--crexe-self-test") return Logic.Add(20,22)==42 && Logic.Add(-3,2)==-1 ? 0:1;
        try { using var check = new GraphicsCheck(); check.Run(); return 0; }
        catch (Exception error) { Console.Error.WriteLine(error); return 1; }
    }
}
