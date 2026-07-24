// ============================================================
// main.rs — point d'entrée du kernel yoshi_os
// ============================================================

// On désactive la bibliothèque standard de Rust : elle suppose la présence
// d'un OS en dessous (allocation mémoire, threads, fichiers...) qu'on n'a pas.
// On garde seulement `core`, la partie de la stdlib qui ne dépend d'aucun OS.
#![no_std]

// On désactive le point d'entrée Rust classique (`fn main()`), qui dépend
// du runtime C fourni normalement par l'OS. On va fournir notre propre
// point d'entrée bas niveau à la place (`_start`, plus bas).
#![no_main]

// On importe notre module vga_buffer (défini dans src/vga_buffer.rs).
// C'est ici qu'on a mis toute la logique d'écriture à l'écran.
mod vga_buffer;

// PanicInfo : le type que Rust utilise pour décrire une panique
// (message, fichier, ligne). Vient de `core`, pas de `std`.
use core::panic::PanicInfo;

// Sans OS, Rust ne sait pas quoi faire en cas de panique (normalement,
// std affiche le message et termine le process). On doit donc fournir
// nous-mêmes ce comportement via #[panic_handler].
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // On utilise notre propre macro println! (définie dans vga_buffer.rs)
    // pour afficher le message de panique et sa localisation à l'écran.
    println!("{}", info);

    // -> ! (never type) : cette fonction ne retourne jamais.
    // Une panique dans un kernel n'a nulle part où "revenir" -> boucle infinie.
    loop {}
}

// #[unsafe(no_mangle)] : empêche le compilateur de renommer cette fonction
// en interne (name mangling). Sans ça, le linker ne retrouverait pas le
// symbole "_start" tel quel.
#[unsafe(no_mangle)]
// extern "C" : utilise la convention d'appel C, celle attendue par le
// bootloader / le linker pour un point d'entrée bas niveau.
// -> ! : ce point d'entrée ne retourne jamais non plus (pas d'OS à qui
// "revenir" une fois le kernel démarré).
pub extern "C" fn _start() -> ! {
    // println! vient de notre module vga_buffer (macro exportée à la racine
    // de la crate via #[macro_export], donc utilisable directement ici sans
    // "use" explicite).
    println!("Hello World{}", "!");

    // Boucle infinie : on n'a pas encore de scheduler / process à qui rendre
    // la main, donc le kernel tourne à vide pour toujours après l'affichage.
    loop {}
}