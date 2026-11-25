#import <Cocoa/Cocoa.h>
#include <OpenGL/gl3.h>
#include <cstring>
#include <cstdio>
int add(int,int);
int main(int argc,char **argv) {
    if(argc>1 && std::strcmp(argv[1],"--crexe-self-test")==0) return add(20,22)==42 && add(-3,2)==-1 ? 0:1;
    @autoreleasepool {
        [NSApplication sharedApplication];
        NSOpenGLPixelFormatAttribute attributes[]={NSOpenGLPFAOpenGLProfile,NSOpenGLProfileVersion3_2Core,NSOpenGLPFAColorSize,24,0};
        NSOpenGLPixelFormat *format=[[NSOpenGLPixelFormat alloc] initWithAttributes:attributes];
        if(!format) return 2;
        NSOpenGLView *view=[[NSOpenGLView alloc] initWithFrame:NSMakeRect(0,0,320,240) pixelFormat:format];
        if(!view.openGLContext) return 3;
        [view.openGLContext makeCurrentContext];
        const char *source="#version 150 core\nvoid main(){gl_Position=vec4(0,0,0,1);}";
        GLuint shader=glCreateShader(GL_VERTEX_SHADER); glShaderSource(shader,1,&source,nullptr); glCompileShader(shader);
        GLint ok=0; glGetShaderiv(shader,GL_COMPILE_STATUS,&ok);
        glClearColor(0.2f,0.8f,0.3f,1); glClear(GL_COLOR_BUFFER_BIT); glFinish();
        int result=ok && glGetError()==GL_NO_ERROR ? 0:1;
        if(!result) std::puts("CREXE_GRAPHICS_OK");
        glDeleteShader(shader); [NSOpenGLContext clearCurrentContext]; return result;
    }
}
