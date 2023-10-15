/**
 * This file is a module that gets imported for use in the module_import example
 * it includes several functions and constants that get exported for use
 */

export const MY_FAVOURITE_FOOD = 'saskatoonberries';

let book_list = [];
export function addBook(title) {
    book_list.push(title)
}
export function listBooks() {
    return book_list;
}