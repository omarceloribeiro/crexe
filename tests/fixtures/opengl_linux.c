#include <gtk/gtk.h>
#include <epoxy/gl.h>
#include <string.h>
#include <stdio.h>
int add(int,int);
static int result=1;
static gboolean render(GtkGLArea *area, GdkGLContext *context, gpointer data) {
    (void)context; (void)data;
    if(gtk_gl_area_get_error(area)) { gtk_main_quit(); return TRUE; }
    const char *source="#version 330 core\nvoid main(){gl_Position=vec4(0,0,0,1);}";
    GLuint shader=glCreateShader(GL_VERTEX_SHADER); glShaderSource(shader,1,&source,NULL); glCompileShader(shader);
    GLint ok=0; glGetShaderiv(shader,GL_COMPILE_STATUS,&ok);
    glClearColor(0.2f,0.8f,0.3f,1); glClear(GL_COLOR_BUFFER_BIT); glFinish();
    result=ok && glGetError()==GL_NO_ERROR ? 0:1;
    if(!result) puts("CREXE_GRAPHICS_OK");
    glDeleteShader(shader); gtk_main_quit(); return TRUE;
}
static gboolean timeout_check(gpointer data) { (void)data; gtk_main_quit(); return G_SOURCE_REMOVE; }
int main(int argc,char **argv) {
    if(argc>1 && strcmp(argv[1],"--crexe-self-test")==0) return add(20,22)==42 && add(-3,2)==-1 ? 0:1;
    if(!gtk_init_check(&argc,&argv)) return 2;
    GtkWidget *window=gtk_window_new(GTK_WINDOW_TOPLEVEL), *area=gtk_gl_area_new();
    gtk_window_set_default_size(GTK_WINDOW(window),320,240);
    gtk_gl_area_set_required_version(GTK_GL_AREA(area),3,3);
    g_signal_connect(area,"render",G_CALLBACK(render),NULL);
    gtk_container_add(GTK_CONTAINER(window),area); gtk_widget_show_all(window);
    g_timeout_add_seconds(10,timeout_check,NULL); gtk_main(); gtk_widget_destroy(window); return result;
}
