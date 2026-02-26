async function search() {
    const field = document.getElementById("field").value;
    const value = document.getElementById("value").value;

    const response = await fetch("http://localhost:3000/query", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({ field, value })
    });

    const data = await response.json();

    const resultsList = document.getElementById("results");
    resultsList.innerHTML = "";

    if (data.length === 0) {
        resultsList.innerHTML = "<li>No results found</li>";
        return;
    }

    data.forEach(book => {
        const li = document.createElement("li");
        li.textContent = `"${book.title}" by ${book.author} (ISBN: ${book.isbn})`;
        resultsList.appendChild(li);
    });
}

document.getElementById("addBookForm").addEventListener("submit", async (e) => {
    e.preventDefault();

    const book = {
        title: document.getElementById("title").value,
        author: document.getElementById("author").value,
        isbn: document.getElementById("isbn").value
    };

    try {
        const response = await fetch("http://localhost:3000/books", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify(book)
        });

        const result = await response.json();

        document.getElementById("addResult").innerText = result;
    } catch (error) {
        document.getElementById("addResult").innerText = "Error adding book";
    }
});
