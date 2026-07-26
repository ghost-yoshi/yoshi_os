use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

use volatile::Volatile;

// ------------------------------------------------------------
// Couleurs
// ------------------------------------------------------------

#[allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// disposition mémoire qu'un simple u8 (aucun octet de padding ajouté).
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        // Encodage : 4 bits de fond (décalés de 4 vers la gauche) + 4 bits
        // de premier plan, combinés avec un OR bit à bit.
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// ------------------------------------------------------------
// Tampon de texte
// ------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

// Dimensions fixes de l'écran en mode texte VGA : 25 lignes, 80 colonnes.
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// repr(transparent) : Buffer doit avoir exactement la même disposition
// mémoire que son unique champ (le tableau 2D de ScreenChar).
#[repr(transparent)]
struct Buffer {
    // Tableau 2D : chaque case est enveloppée dans Volatile pour empêcher
    // les optimisations dangereuses du compilateur.
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// ------------------------------------------------------------
// Writer
// ------------------------------------------------------------

pub struct Writer {
    // Position actuelle dans la dernière ligne (colonne courante d'écriture)
    column_position: usize,
    // Couleur utilisée pour les prochains caractères écrits
    color_code: ColorCode,
    // Référence vers le VGA buffer. 'static : valide pendant toutue la durée
    // du programme (vrai ici puisque le hardware existe tout du long).
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            // Cas du retour à la ligne : pas d'affichage, on déplace juste
            // le curseur logique vers la ligne suivante.
            b'\n' => self.new_line(),
            // Tout autre octet : on l'affiche réellement à l'écran.
            byte => {
                // Si la ligne courante est pleine, on passe à la ligne
                // suivante avant d'écrire (le "wrap" horizontal).
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                // On écrit toujours sur la dernière ligne visible.
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    // Raccourci Rust : équivalent à `color_code: color_code`
                    color_code,
                });
                // On avance le curseur logique d'une colonne.
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        // On parcourt la chaîne octet par octet (pas caractère Unicode,
        // car le VGA buffer ne comprend que de l'ASCII/code page 437).
        for byte in s.bytes() {
            match byte {
                // Plage ASCII imprimable (espace à ~) ou retour à la ligne
                // -> affichage normal.
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                // .read() : lit la valeur actuelle (via Volatile, pour les
                // mêmes raisons que .write() plus haut).
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        // On efface la dernière ligne (qui vient de se "libérer") pour
        // qu'elle soit prête à recevoir du nouveau texte.
        self.clear_row(BUFFER_HEIGHT - 1);
        // On revient au début de la ligne pour la prochaine écriture.
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        // Un caractère "espace" avec la couleur courante : la façon la
        // plus simple d'effacer visuellement une ligne.
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// ------------------------------------------------------------
// Interface globale (WRITER statique)
// ------------------------------------------------------------

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        // Seul bloc unsafe de tout le module : on convertit l'adresse
        // brute 0xb8000 en référence &'static mut Buffer. Après cette
        // ligne, toutes les opérations passent par des vérifications
        // normales du compilateur (bounds checking inclus).
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

// ------------------------------------------------------------
// Macros print! et println!
// ------------------------------------------------------------


#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    // Cas sans argument : juste un retour à la ligne.
    () => ($crate::print!("\n"));
    // Cas avec arguments (ex: println!("x = {}", x)) : on délègue à print!
    // en ajoutant un \n à la fin.
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}


#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // Nécessaire pour pouvoir appeler write_fmt (méthode du trait fmt::Write)
    use core::fmt::Write;
    // .lock() : acquiert le spinlock du Mutex avant d'écrire (empêche deux
    // écritures simultanées de se marcher dessus).
    // .unwrap() : write_fmt renvoie un Result: on considère que l'écriture
    // VGA ne peut jamais échouer, donc unwrap() ne devrait jamais paniquer.
    WRITER.lock().write_fmt(args).unwrap();
}