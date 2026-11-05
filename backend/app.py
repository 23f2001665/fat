from __future__ import annotations

import os

from .application import create_app
from .config import Config


app = create_app(Config)


if __name__ == "__main__":
    app.run(
        host=os.getenv("FAT_HOST", "127.0.0.1"),
        port=int(os.getenv("FAT_PORT", "5000")),
        debug=os.getenv("FLASK_DEBUG", "0") == "1",
    )
