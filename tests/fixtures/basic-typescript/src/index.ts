import { helper } from "./helper";

export interface Named {
    name: string;
}

export class Widget implements Named {
    name = "widget";

    render(): string {
        return helper(this.name);
    }
}

export function build(): Widget {
    return new Widget();
}
