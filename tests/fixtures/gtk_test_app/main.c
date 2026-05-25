/*
 * GTK Test Application for AT-SPI2 E2E Testing
 *
 * Simple GTK3 window with labeled widgets for testing:
 * - Window title: "AT-SPI2 Test App"
 * - Button with accessible name: "Test Button"
 * - Entry with accessible name: "Test Entry"
 * - Label with text: "Test Label"
 *
 * Build: make
 * Run: GTK_MODULES=gail:atk-bridge ./gtk_test_app
 */

#include <gtk/gtk.h>
#include <atk/atk.h>

static void on_button_clicked(GtkWidget *widget, gpointer data) {
    g_print("Button clicked\n");
}

static void set_accessible_name(GtkWidget *widget, const char *name) {
    AtkObject *accessible = gtk_widget_get_accessible(widget);
    if (accessible) {
        atk_object_set_name(accessible, name);
    }
    gtk_widget_set_name(widget, name);
}

int main(int argc, char *argv[]) {
    GtkWidget *window;
    GtkWidget *box;
    GtkWidget *button;
    GtkWidget *entry;
    GtkWidget *label;

    gtk_init(&argc, &argv);

    /* Create main window */
    window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "AT-SPI2 Test App");
    gtk_window_set_default_size(GTK_WINDOW(window), 300, 200);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

    /* Create vertical box container */
    box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 10);
    gtk_container_set_border_width(GTK_CONTAINER(box), 20);
    gtk_container_add(GTK_CONTAINER(window), box);

    /* Create label */
    label = gtk_label_new("Test Label");
    set_accessible_name(label, "Test Label");
    gtk_box_pack_start(GTK_BOX(box), label, FALSE, FALSE, 0);

    /* Create entry */
    entry = gtk_entry_new();
    gtk_entry_set_placeholder_text(GTK_ENTRY(entry), "Type here...");
    set_accessible_name(entry, "Test Entry");
    gtk_box_pack_start(GTK_BOX(box), entry, FALSE, FALSE, 0);

    /* Create button */
    button = gtk_button_new_with_label("Click Me");
    set_accessible_name(button, "Test Button");
    g_signal_connect(button, "clicked", G_CALLBACK(on_button_clicked), NULL);
    gtk_box_pack_start(GTK_BOX(box), button, FALSE, FALSE, 0);

    /* Show all widgets */
    gtk_widget_show_all(window);

    /* Run GTK main loop */
    gtk_main();

    return 0;
}
