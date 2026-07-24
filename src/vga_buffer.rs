// ============================================================
// vga_buffer.rs — module d'écriture dans le tampon texte VGA
// ============================================================

// Nécessaire pour implémenter le trait fmt::Write (utilisé par write!/println!)
use core::fmt;

// lazy_static! : permet de créer un `static` dont l'initialisation se fait
// au premier accès (à l'exécution) plutôt qu'à la compilation. Nécessaire
// car ColorCode::new(...) et le déréférencement de pointeur brut ne sont
// pas calculables par le "const evaluator" de Rust.
use lazy_static::lazy_static;

// Mutex "spinlock" : verrou basique qui ne nécessite aucune fonctionnalité
// de l'OS (contrairement à std::sync::Mutex qui a besoin de threads/OS
// pour mettre en pause un thread en attente).
use spin::Mutex;

// Volatile<T> : force le compilateur à ne JAMAIS optimiser/supprimer les
// lectures/écritures sur cette valeur, même s'il pense (à tort) qu'elles
// sont inutiles. Indispensable ici car on écrit dans une zone mémoire
// hardware, pas de la RAM normale.
use volatile::Volatile;

// ------------------------------------------------------------
// Couleurs
// ------------------------------------------------------------

// #[allow(dead_code)] : désactive les warnings "variante jamais utilisée"
// (on ne se sert pas forcément de toutes les couleurs tout de suite).
#[allow(dead_code)]
// derive : génère automatiquement des implémentations utiles.
// Debug -> affichable avec {:?}, Clone/Copy -> copiable simplement,
// PartialEq/Eq -> comparable avec ==.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// repr(u8) : force chaque variante à être stockée sur exactement 1 octet,
// avec la valeur numérique explicite qu'on donne (Black = 0, Blue = 1...).
// Nécessaire car le format VGA attend précisément 4 bits par couleur.
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

// Newtype : on enveloppe un simple u8 dans un type nommé, pour que le
// compilateur distingue "un octet qui représente un code couleur" d'un
// u8 quelconque ailleurs dans le code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// repr(transparent) : garantit que ColorCode a exactement la même
// disposition mémoire qu'un simple u8 (aucun octet de padding ajouté).
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        // Encodage : 4 bits de fond (décalés de 4 vers la gauche) + 4 bits
        // de premier plan, combinés avec un OR bit à bit.
        // Ex: background=Black(0000), foreground=Yellow(1110)
        //     -> 0000_1110 en binaire = l'octet couleur complet
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// ------------------------------------------------------------
// Tampon de texte
// ------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// repr(C) : force l'ordre des champs à être exactement celui écrit ici
// (ascii_character puis color_code), comme le ferait un compilateur C.
// Par défaut, Rust est libre de réordonner les champs d'une struct -
// invisible et inoffensif en temps normal, mais catastrophique ici car
// le hardware VGA attend ce format précis : [caractère][couleur].
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
    // les optimisations dangereuses du compilateur (voir plus haut).
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
    // Référence vers le VGA buffer. 'static : valide pendant toute la durée
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
                // .write(...) au lieu d'une affectation `=` : passe par
                // Volatile pour garantir que l'écriture n'est jamais
                // supprimée par une optimisation du compilateur.
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
                // Tout octet hors de cette plage (ex: un des 2 octets d'un
                // caractère UTF-8 multi-octets comme 'ö') -> on affiche un
                // caractère de remplacement '■' (code 0xfe sur le hardware
                // VGA), plutôt que d'afficher un octet incompréhensible.
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        // On décale chaque ligne d'une position vers le haut : la ligne 1
        // devient la ligne 0, la ligne 2 devient la ligne 1, etc.
        // On saute la ligne 0 dans la boucle car c'est elle qui "sort" de
        // l'écran (on ne l'écrit nulle part, elle est juste perdue).
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

// Implémentation du trait core::fmt::Write : c'est ce qui permet d'utiliser
// les macros standard write!/writeln! avec notre Writer (support de {},
// {:?}, formatage de nombres, etc., sans les réécrire nous-mêmes).
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
    // WRITER : instance globale unique du Writer, utilisable depuis
    // n'importe où dans le kernel (via WRITER.lock()).
    //
    // Mutex<Writer> : ajoute une mutabilité intérieure sûre - sans lui,
    // un `static` serait immuable par défaut et on ne pourrait jamais
    // appeler les méthodes &mut self de Writer dessus.
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
// Macros print! / println!
// ------------------------------------------------------------

// #[macro_export] : rend la macro disponible dans toute la crate (pas
// seulement ce module), et la place à la racine (d'où `$crate::print!`
// plus bas, pour que println! fonctionne sans avoir à importer print!
// séparément).
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

// #[doc(hidden)] : cache cette fonction de la documentation générée -
// c'est un détail d'implémentation interne aux macros, pas une API
// destinée à être appelée directement ailleurs (même si elle doit être
// pub pour que les macros, définies au niveau racine, puissent l'appeler).
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