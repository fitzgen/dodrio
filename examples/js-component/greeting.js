// A rendering component defined in JavaScript!
class Greeting {
  constructor(who) {
    this.who = who;
  }

  render() {
    return {
      tagName: "p",
      attributes: [
        {
          name: "class",
          value: "greeting",
        },
      ],
      listeners: [
        {
          on: "click",
          callback: this.onClick.bind(this),
        }
      ],
      children: [
        "Hello, ",
        {
          tagName: "strong",
          children: [this.who],
        }
      ],
    };
  }

  async onClick(vdom, event) {
    // Be more excited!
    this.who += "!";

    // Schedule a re-render.
    await vdom.render();

    console.log("re-rendering finished!");
  }
}
