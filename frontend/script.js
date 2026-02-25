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
