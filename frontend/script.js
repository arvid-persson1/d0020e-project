async function search() {
    const operator = document.getElementById("operator").value;

    const conditionElements = document.querySelectorAll("#conditions .condition");

    const conditions = [];

    conditionElements.forEach(cond => {
        const field = cond.querySelector(".field").value;
        const value = cond.querySelector(".value").value;

        if (value.trim() !== "") {
            conditions.push({ field, value });
        }
    });

    if (conditions.length === 0) {
        alert("Please enter at least one search condition.");
        return;
    }

    const response = await fetch("http://localhost:3000/query", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            operator,
            conditions
        })
    });

    const data = await response.json();

    const resultsList = document.getElementById("results");
    resultsList.innerHTML = "";

    if (data.length === 0) {
        resultsList.innerHTML = "<li>No results found</li>";
        return;
    }

    data.forEach(result => {
        const li = document.createElement("li");
        const book = result.item;
        li.textContent = `"${book.title}" by ${book.author} (ISBN: ${book.isbn}) - Source: ${result.source}`;
        resultsList.appendChild(li);
    });
}

function addCondition() {
    const container = document.getElementById("conditions");

    const div = document.createElement("div");
    div.className = "condition";

    div.innerHTML = `
        <select class="field">
            <option value="author">Author</option>
            <option value="title">Title</option>
            <option value="isbn">ISBN</option>
        </select>

        <input type="text" class="value" placeholder="Enter value" />
    `;

    container.appendChild(div);
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
